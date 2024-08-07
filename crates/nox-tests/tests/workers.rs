/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use connected_client::ConnectedClient;
use created_swarm::{make_swarms, make_swarms_with_cfg};
use eyre::Context;
use hex::FromHex;
use log_utils::enable_logs;
use maplit::hashmap;
use serde_json::{json, Value};
use test_utils::get_default_chain_config;
use workers::CUID;

pub(crate) async fn create_worker(client: &mut ConnectedClient, deal_id: &str) -> String {
    let init_id_1 =
        <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
            .unwrap();
    let unit_ids = vec![init_id_1];
    let data = hashmap! {
        "deal_id" => json!(deal_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "cu_ids" => json!(unit_ids)
    };

    let response = client
        .execute_particle(
            r#"
            (seq
                (call relay ("worker" "create") [deal_id cu_ids] worker_peer_id)
                (call client ("return" "") [worker_peer_id])
            )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let worker_id = response[0].as_str().unwrap().to_string();
    assert_ne!(worker_id.len(), 0);

    worker_id
}

async fn get_worker_id(client: &mut ConnectedClient, deal_id: &str) -> String {
    let data = hashmap! {
        "deal_id" => json!(deal_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string())
    };

    let response = client
        .execute_particle(
            r#"
            (seq
                (call relay ("worker" "get_worker_id") [deal_id] get_worker_peer_id)
                (seq
                    (ap get_worker_peer_id.$.[0] worker_peer_id)
                    (call client ("return" "") [worker_peer_id])
                )
            )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let worker_id = response[0].as_str().unwrap().to_string();
    assert_ne!(worker_id.len(), 0);

    worker_id
}

async fn is_worker_active(client: &mut ConnectedClient, deal_id: &str) -> bool {
    let data = hashmap! {
        "deal_id" => json!(deal_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string())
    };

    let response = client
        .execute_particle(
            r#"
            (seq
                (call relay ("worker" "is_active") [deal_id] is_active)
                (call client ("return" "") [is_active])
            )"#,
            data.clone(),
        )
        .await
        .unwrap();

    response[0].as_bool().unwrap()
}

#[tokio::test]
async fn test_worker_different_deal_ids() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let deal_id_mixed_prefix = "0x1234aBcD";
    let deal_id_mixed = "1234aBcD";
    let deal_id_lowercase_prefix = "0x1234abcd";
    let deal_id_lowercase = "1234abcd";

    let worker_id_1 = create_worker(&mut client, deal_id_mixed_prefix).await;

    let worker_id_2 = get_worker_id(&mut client, deal_id_mixed).await;
    let worker_id_3 = get_worker_id(&mut client, deal_id_lowercase_prefix).await;
    let worker_id_4 = get_worker_id(&mut client, deal_id_lowercase).await;
    let worker_id_5 = get_worker_id(&mut client, deal_id_mixed_prefix).await;

    assert_eq!(worker_id_1, worker_id_2);
    assert_eq!(worker_id_1, worker_id_3);
    assert_eq!(worker_id_1, worker_id_4);
    assert_eq!(worker_id_1, worker_id_5);

    assert!(is_worker_active(&mut client, deal_id_lowercase).await);
    assert!(is_worker_active(&mut client, deal_id_mixed).await);
    assert!(is_worker_active(&mut client, deal_id_lowercase_prefix).await);
    assert!(is_worker_active(&mut client, deal_id_mixed_prefix).await);
}

#[tokio::test]
async fn test_resolve_subnet_on_worker() {
    let deal_id = "0x9DcaFca9B88f49d91c38a32E7d9A86a7d9a37B04";

    enable_logs();
    let script = tokio::fs::read("./tests/workers/test_subnet_resolve_on_worker.air")
        .await
        .wrap_err("read test data")
        .unwrap();
    let script = String::from_utf8(script)
        .wrap_err("decode test data")
        .unwrap();

    // Create a mock
    let mut server = mockito::Server::new_async().await;
    let url = server.url();
    let _mock = server
        .mock("POST", "/")
        .expect(1)
        .with_status(429)
        .with_header("content-type", "application/json")
        .create();

    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.chain_config = Some(get_default_chain_config(&url));
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let worker_id = create_worker(&mut client, deal_id).await;

    let data = hashmap! {
                "-relay-" => json!(swarms[0].peer_id.to_string()),
                "-worker_id-" => json!(worker_id),
                "-deal_id-" => json!(deal_id),
    };

    let result = client
        .execute_particle(script.clone(), data.clone())
        .await
        .wrap_err("execute particle")
        .unwrap();

    let expected = {
        let error = Value::Array(vec![Value::String(
            "RPC error: Request rejected `429`".to_string(),
        )]);
        let mut object_map = serde_json::Map::new();
        object_map.insert("error".to_string(), error);
        object_map.insert("success".to_string(), Value::Bool(false));
        object_map.insert("workers".to_string(), Value::Array(vec![]));

        vec![Value::Object(object_map)]
    };

    assert_eq!(result, expected)
}
