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

#[macro_use]
extern crate fstrings;
use alloy_primitives::hex;
use alloy_sol_types::sol_data::Array;
use alloy_sol_types::SolType;
use chain_connector::SubnetResolveResult;
use chain_data::{parse_peer_id, peer_id_to_bytes};
use connected_client::ConnectedClient;
use created_swarm::{
    make_swarms, make_swarms_with_cfg, make_swarms_with_keypair,
    make_swarms_with_transport_and_mocked_vm,
};
use eyre::{Report, WrapErr};
use fluence_keypair::KeyPair;
use fluence_libp2p::RandomPeerId;
use fluence_libp2p::Transport;
use hex_utils::encode_hex_0x;
use itertools::Itertools;
use json_utils::into_array;
use libp2p::core::Multiaddr;
use libp2p::kad::KBucketKey;
use libp2p::PeerId;
use log_utils::enable_logs;
use maplit::hashmap;
use multihash::Multihash;
use now_millis::now_ms;
use particle_protocol::Particle;
use serde::Deserialize;
use serde_json::{json, Value as JValue};
use service_modules::load_module;
use std::collections::HashMap;
use std::default::Default;
use std::str::FromStr;
use std::time::Duration;
use test_constants::PARTICLE_TTL;
use test_utils::{create_service, get_default_chain_config};

#[derive(Deserialize, Debug)]
struct NodeInfo {
    #[allow(dead_code)]
    pub external_addresses: Vec<Multiaddr>,
}

#[tokio::test]
async fn identify() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"
        (seq
            (call relay ("peer" "identify") [] info)
            (call client ("op" "return") [info])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        )
        .await;

    let info = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let _: NodeInfo = serde_json::from_value(info[0].clone())
        .unwrap_or_else(|_| panic!("deserialize {:?}", info[0]));
}

#[ignore]
#[tokio::test]
async fn big_identity() {
    let swarms = make_swarms_with_transport_and_mocked_vm(1, Transport::Network).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let mut particle = Particle::default();
    particle.init_peer_id = client.peer_id;
    particle.data = (0..(1024 * 1024 * 20)).map(|_| u8::MAX).collect();
    particle.timestamp = now_ms() as u64;
    particle.ttl = PARTICLE_TTL;
    client.send(particle).await;

    client.timeout = Duration::from_secs(60);
    client.receive().await.wrap_err("receive").unwrap();
}

#[tokio::test]
async fn remove_service() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (call relay ("srv" "remove") [service])
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after])
            )
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    use serde_json::Value::Array;

    if let [Array(before), Array(after)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected two arrays")
    }
}

#[tokio::test]
async fn remove_service_restart() {
    let kp = KeyPair::generate_ed25519();
    let manager_kp = KeyPair::generate_ed25519();

    let swarm_kp = kp.clone();
    let swarm_manager_kp = manager_kp.clone();
    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.keypair = swarm_kp.clone();
        cfg.management_keypair = swarm_manager_kp.clone();
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(manager_kp.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (call relay ("srv" "remove") [service])
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after])
            )
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    use serde_json::Value::Array;

    if let [Array(before), Array(after)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected two arrays")
    }

    // stop swarm
    swarms
        .into_iter()
        .map(|s| s.exit_outlet.send(()))
        .for_each(drop);
    let swarm_kp = kp.clone();
    let swarm_manager_kp = manager_kp.clone();
    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.keypair = swarm_kp.clone();
        cfg.management_keypair = swarm_manager_kp.clone();
        cfg
    })
    .await;
    let mut client =
        ConnectedClient::connect_with_keypair(swarms[0].multiaddr.clone(), Some(manager_kp))
            .await
            .wrap_err("connect client")
            .unwrap();

    client
        .send_particle(
            r#"
        (seq
            (call relay ("srv" "list") [] list_after)
            (call %init_peer_id% ("op" "return") [list_after])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    if let [Array(after)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected array")
    }
}

#[tokio::test]
async fn remove_service_by_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (seq
                    (call relay ("srv" "list") [] list_before)
                    (call relay ("srv" "remove") [alias])
                )
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after])
            )
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    use serde_json::Value::Array;

    if let [Array(before), Array(after)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 0);
    } else {
        panic!("incorrect args: expected two arrays")
    }
}

#[tokio::test]
async fn non_owner_remove_service() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    let args = client2
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "list") [] list_before)
                (xor
                    (call relay ("srv" "remove") [service])
                    (ap %last_error%.$.instruction error)
                )
            )
            (seq
                (call relay ("srv" "list") [] list_after)
                (call %init_peer_id% ("op" "return") [list_before list_after error])
            )
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await
        .unwrap();

    use serde_json::Value::{Array, String};

    if let [Array(before), Array(after), String(error)] = args.as_slice() {
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
        assert!(!error.is_empty());

        assert_eq!(error, r#"call relay ("srv" "remove") [service] "#);
    } else {
        panic!("incorrect args: expected two arrays, got: {:?}", args)
    }
}

#[tokio::test]
async fn resolve_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call relay ("srv" "resolve_alias") [alias] result)
            )
            (call %init_peer_id% ("op" "return") [result])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    let service_id = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let service_id = service_id.into_iter().next().unwrap();
    let service_id: String = serde_json::from_value(service_id).unwrap();

    assert_eq!(tetraplets_service.id, service_id);
}

#[tokio::test]
async fn resolve_alias_not_exists() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"
        (xor
            (seq
                (call relay ("srv" "resolve_alias") [alias] result)
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    let error = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let error = error.into_iter().next().unwrap();
    let failed_instruction = error.as_str().unwrap();
    assert_eq!(
        failed_instruction,
        r#"call relay ("srv" "resolve_alias") [alias] result"#
    );
}

#[tokio::test]
async fn resolve_alias_opt() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call relay ("srv" "resolve_alias_opt") [alias] result)
            )
            (call %init_peer_id% ("op" "return") [result.$.[0]!])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    let service_id = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let service_id_opt = service_id.into_iter().next().unwrap();
    let service_id: String = serde_json::from_value(service_id_opt).unwrap();

    assert_eq!(tetraplets_service.id, service_id);
}

#[tokio::test]
async fn resolve_alias_opt_not_exists() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"
        (xor
            (seq
                (call relay ("srv" "resolve_alias_opt") [alias] result)
                (call %init_peer_id% ("op" "return") [result])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    let result = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let result = result.into_iter().next().unwrap();
    let result: Vec<String> = serde_json::from_value(result).unwrap();

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn resolve_alias_removed() {
    enable_logs();
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (xor
            (seq
                (seq
                    (call relay ("srv" "add_alias") [alias service])
                    (call relay ("srv" "remove") [alias])
                )
                (call relay ("srv" "resolve_alias") [alias] result)
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!("some_alias".to_string()),
            },
        )
        .await;

    let error = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let error = error.into_iter().next().unwrap();
    let failed_instruction = error.as_str().unwrap();
    assert_eq!(
        failed_instruction,
        r#"call relay ("srv" "resolve_alias") [alias] result"#
    );
}

#[tokio::test]
async fn timestamp_ms() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"
        (seq
            (call relay ("peer" "timestamp_ms") [] result)
            (call client ("op" "return") [result])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        )
        .await;

    let result = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let result = result.into_iter().next().unwrap();
    let _: u64 = serde_json::from_value(result).unwrap();
}

#[tokio::test]
async fn timestamp_sec() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] result)
            (call client ("op" "return") [result])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        )
        .await;

    let result = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
    let result = result.into_iter().next().unwrap();
    let _: u64 = serde_json::from_value(result).unwrap();
}

#[tokio::test]
async fn base58_string_builtins() {
    let script = r#"
    (seq
        (call relay ("op" "string_to_b58") [string] b58_string_out)
        (seq
            (call relay ("op" "string_from_b58") [b58_string] string_out)
            (call relay ("op" "string_from_b58") [b58_string_out] identity_string)
        )
    )
    "#;

    let string = "hello, this is a string! ДОБРЫЙ ВЕЧЕР КАК СЛЫШНО";
    let b58_string = bs58::encode(string).into_string();
    let args = hashmap! {
        "string" => json!(string),
        "b58_string" => json!(b58_string),
    };

    let result = exec_script(script, args, "b58_string_out string_out identity_string", 1)
        .await
        .unwrap();
    assert_eq!(result[0], JValue::String(b58_string));
    assert_eq!(result[1], JValue::String(string.into()));
    assert_eq!(result[2], JValue::String(string.into()));
}

#[tokio::test]
async fn base58_bytes_builtins() {
    let script = r#"
    (seq
        (call relay ("op" "bytes_to_b58") [bytes] b58_string_out)
        (seq
            (call relay ("op" "bytes_from_b58") [b58_string] bytes_out)
            (call relay ("op" "bytes_from_b58") [b58_string_out] identity_bytes)
        )
    )
    "#;

    let bytes: Vec<_> = (1..32).map(|i| (200 + i) as u8).collect();
    let b58_string = bs58::encode(&bytes).into_string();
    let args = hashmap! {
        "b58_string" => json!(b58_string),
        "bytes" => json!(bytes),
    };

    let result = exec_script(script, args, "b58_string_out bytes_out identity_bytes", 1)
        .await
        .unwrap();
    // let array = into_array(result).expect("must be an array");
    assert_eq!(result[0], json!(b58_string));
    assert_eq!(result[1], json!(bytes));
    assert_eq!(result[2], json!(bytes));
}

#[tokio::test]
async fn sha256() {
    let script = r#"
    (seq
        (seq
            ; hash string to multihash encoded as base58
            (call relay ("op" "sha256_string") [string] string_mhash)
            ; hash string to sha256 digest encoded as base58
            (call relay ("op" "sha256_string") [string true] string_digest)
        )
        (seq
            ; hash string to multihash encoded as byte array
            (call relay ("op" "sha256_string") [string false true] bytes_mhash)
            ; hash string to sha256 digest encoded as byte array
            (call relay ("op" "sha256_string") [string true true] bytes_digest)
        )
    )
    "#;

    let string = "hello, как слышно? ХОРОШО!";
    let sha_256: Multihash<64> = Multihash::wrap(0x12, string.as_bytes()).unwrap();
    let args = hashmap! {
        "string" => json!(string),
    };

    let result = exec_script(
        script,
        args,
        "string_mhash string_digest bytes_mhash bytes_digest",
        1,
    )
    .await
    .unwrap();

    // multihash as base58
    assert_eq!(
        result[0],
        json!(bs58::encode(sha_256.to_bytes()).into_string())
    );
    // sha256 digest as base58
    assert_eq!(
        result[1],
        json!(bs58::encode(sha_256.digest()).into_string())
    );
    // multihash as byte array
    assert_eq!(result[2], json!(sha_256.to_bytes()));
    // sha256 digest as byte array
    assert_eq!(result[3], json!(sha_256.digest()));
}

#[tokio::test]
async fn neighborhood() {
    let script = r#"
    (seq
        (seq
            (seq
                (call relay ("op" "string_to_b58") ["key"] key)
                (call relay ("kad" "neighborhood") [key] neighborhood_by_key)
            )
            (seq
                (call relay ("op" "sha256_string") ["key"] mhash)
                (call relay ("kad" "neighborhood") [mhash true] neighborhood_by_mhash)
            )
        )
        (xor
            (call relay ("kad" "neighborhood") [key true])
            (ap %last_error%.$.message error)
        )
    )
    "#;

    let mut result = exec_script(
        script,
        <_>::default(),
        "neighborhood_by_key neighborhood_by_mhash error",
        2,
    )
    .await
    .unwrap();
    let neighborhood_by_key = into_array(result[0].take())
        .expect("neighborhood is an array")
        .into_iter()
        .map(|v| PeerId::from_str(v.as_str().expect("peerid is string")).expect("peerid is valid"));
    let neighborhood_by_mhash = into_array(result[1].take())
        .expect("neighborhood is an array")
        .into_iter()
        .map(|v| PeerId::from_str(v.as_str().expect("peerid is string")).expect("peerid is valid"));
    let error = result[2].take();
    let error = error.as_str().expect("error is string");
    assert_eq!(neighborhood_by_key.len(), 1);
    assert_eq!(neighborhood_by_mhash.len(), 1);
    assert!(error.contains("Invalid multihash"));
}

#[tokio::test]
async fn kad_merge() {
    let target = RandomPeerId::random();
    let left = (1..10).map(|_| RandomPeerId::random()).collect::<Vec<_>>();
    let mut right = (1..10).map(|_| RandomPeerId::random()).collect::<Vec<_>>();
    right = right.into_iter().chain(left.clone().into_iter()).collect();
    let count = 10;

    let script = r#"
    (call relay ("kad" "merge") [target left right count] merged)
    "#;

    let args = hashmap! {
        "target" => json!(target.to_base58()),
        "left" => json!(left.iter().map(|id| id.to_base58()).collect::<Vec<_>>()),
        "right" => json!(right.iter().map(|id| id.to_base58()).collect::<Vec<_>>()),
        "count" => json!(count),
    };

    let result = exec_script(script, args, "merged", 1).await.unwrap();
    let merged = result.into_iter().next().expect("merged is defined");
    let merged = into_array(merged).expect("merged is an array");
    let merged = merged
        .into_iter()
        .map(|id| {
            PeerId::from_str(id.as_str().expect("peerid is a string")).expect("peerid is correct")
        })
        .collect::<Vec<_>>();

    let target_key = KBucketKey::from(target);
    let mut expected = left;
    expected.append(&mut right);
    expected = expected.into_iter().unique().collect();
    expected.sort_by_cached_key(|id| target_key.distance(&KBucketKey::from(*id)));
    expected.truncate(count);

    assert_eq!(expected, merged);
}

#[tokio::test]
async fn noop() {
    let result = exec_script(
        r#"(call relay ("op" "noop") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    )
    .await
    .unwrap();
    assert_eq!(result, vec![json!("")])
}

#[tokio::test]
async fn identity() {
    let result = exec_script(
        r#"(call relay ("op" "identity") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    )
    .await
    .unwrap();
    assert_eq!(result, vec![json!("hi")]);

    let error = exec_script(
        r#"
        (xor
            (call relay ("op" "identity") ["hi" "there"] result)
            (ap %last_error%.$.message error)
        )
        "#,
        <_>::default(),
        "error",
        1,
    )
    .await
    .unwrap();

    let error = error[0].as_str().unwrap();
    assert!(error.contains("identity accepts up to 1 arguments, received 2 arguments"));
}

#[tokio::test]
async fn array() {
    let result = exec_script(
        r#"(call relay ("op" "array") ["hi"] result)"#,
        <_>::default(),
        "result",
        1,
    )
    .await
    .unwrap();
    log::info!("result {:?}", result);
    assert_eq!(result, vec![json!(["hi"])]);
}

#[tokio::test]
async fn concat() {
    let result = exec_script(
        r#"(call relay ("op" "concat") [zerozero one empty two three fourfive empty] result)"#,
        hashmap! {
            "zerozero" => json!([0, 0]),
            "empty" => json!([]),
            "one" => json!([1]),
            "two" => json!([2]),
            "three" => json!([3]),
            "fourfive" => json!([4,5]),
        },
        "result",
        1,
    )
    .await
    .unwrap();
    assert_eq!(result, vec![json!([0, 0, 1, 2, 3, 4, 5])])
}

#[tokio::test]
async fn array_length() {
    let result = exec_script(
        r#"
        (seq
            (seq
                (call relay ("op" "array_length") [empty_array] zero)
                (call relay ("op" "array_length") [five_array] five)
            )
            (seq
                (xor
                    (call relay ("op" "array_length") [])
                    (ap %last_error%.$.message zero_error)
                )
                (seq
                    (xor
                        (call relay ("op" "array_length") [empty_array five_array])
                        (ap %last_error%.$.message count_error)
                    )
                    (xor
                        (call relay ("op" "array_length") ["hola"])
                        (ap %last_error%.$.message type_error)
                    )
                )
            )
        )
        "#,
        hashmap! {
            "empty_array" => json!([]),
            "five_array" => json!([1, 2, 3, 4, 5])
        },
        "zero five zero_error count_error type_error",
        1,
    )
    .await
    .unwrap();

    assert_eq!(result, vec![
        json!(0),
        json!(5),
        json!("Local service error, ret_code is 1, error message is '\"op array_length accepts exactly 1 argument: 0 found\"'"),
        json!("Local service error, ret_code is 1, error message is '\"op array_length accepts exactly 1 argument: 2 found\"'"),
        json!("Local service error, ret_code is 1, error message is '\"op array_length's argument must be an array\"'"),
    ])
}

#[tokio::test]
async fn empty_array_slice() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ empty_data sidx eidx ] result)"#,
        hashmap! {
            "empty_data" => json!(Vec::<JValue>::new()),
            "sidx"       => json!(0),
            "eidx"       => json!(2),
        },
        "result",
        1,
    )
    .await
    .unwrap();
    assert_eq!(result[0], json!(Vec::<JValue>::new()));
}

#[tokio::test]
async fn not_array_slice() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ data sidx eidx ] result)"#,
        hashmap! {
            "data"      => json!(1),
            "sidx"      => json!(0),
            "eidx"      => json!(2),
        },
        "result",
        1,
    )
    .await;
    assert!(result.is_err());
    assert!(
        format!("{result:?}").contains("first argument must be an array, was 1"),
        "{}",
        "{result:?}"
    );
}

#[tokio::test]
async fn array_slice_wrong_args_count() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ eidx sidx ] result)"#,
        hashmap! {
            "data"      => json!(vec![1,2,3,4]),
            "sidx"      => json!(0),
            "eidx"      => json!(2),
        },
        "result",
        1,
    )
    .await;
    assert!(result.is_err());
    assert!(format!("{result:?}")
        .contains("invalid number of parameters. need array, start index and end index"));
}

#[tokio::test]
async fn array_slice_out_of_bounds() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ data eidx sidx ] result)"#,
        hashmap! {
            "data" => json!(vec![1,2,3,4]),
            "sidx"       => json!(0),
            "eidx"       => json!(2),
        },
        "result",
        1,
    )
    .await;
    assert!(result.is_err());
    assert!(
        format!("{result:?}")
            .contains("slice indexes out of bounds. start index: 2, end index: 0, array length: 4"),
        "{}",
        "result is {result:?}"
    );
}

#[tokio::test]
async fn array_slice_wrong_second_arg() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ data bad_idx eidx ] result)"#,
        hashmap! {
            "data"      => json!(vec![1,2,3,4]),
            "bad_idx"   => json!(-1),
            "eidx"      => json!(2),
        },
        "result",
        1,
    )
    .await;
    assert!(result.is_err());
    assert!(
        format!("{result:?}")
            .contains("second argument (start index) must be an unsigned integer, was -1"),
        "{}",
        "{result:?}"
    );
}

#[tokio::test]
async fn array_slice_wrong_third_arg() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ data sidx bad_idx] result)"#,
        hashmap! {
            "data"      => json!(vec![1,2,3,4]),
            "bad_idx"   => json!(-1),
            "sidx"      => json!(2),
        },
        "result",
        1,
    )
    .await;
    assert!(result.is_err());
    assert!(
        format!("{result:?}")
            .contains("third argument (end index) must be an unsigned integer, was -1"),
        "{}",
        "{result:?}"
    );
}

#[tokio::test]
async fn array_slice() {
    let result = exec_script(
        r#"(call relay ("array" "slice") [ data sidx eidx ] result)"#,
        hashmap! {
            "data"      => json!(vec![1,2,3,4]),
            "sidx"      => json!(0),
            "eidx"      => json!(2),
        },
        "result",
        1,
    )
    .await
    .unwrap();

    let expected = vec![json!(vec![1, 2])];
    assert_eq!(result, expected);
}

#[tokio::test]
async fn timeout_race() {
    let fast_result = exec_script(
        r#"
        (seq
            (par
                (call relay ("peer" "timeout") [1000 "slow_result"] $result)
                (call relay ("op" "identity") ["fast_result"] $result)
            )
            (canon relay $result #result)
        )
    "#,
        <_>::default(),
        "#result.$[0]",
        1,
    )
    .await
    .unwrap();

    assert_eq!(&fast_result[0], "fast_result");
}

#[tokio::test]
async fn timeout_wait() {
    let slow_result = exec_script(
        r#"
        (seq
            (seq
                (seq
                    (par
                        (call relay ("peer" "timeout") [1000 "timed_out"] $ok_or_err)
                        (call "invalid_peer" ("op" "identity") ["never"] $ok_or_err)
                    )
                    (canon %init_peer_id% $ok_or_err #ok_or_err)
                )
                (xor
                    (match #ok_or_err.$[0] "timed_out"
                        (ap "timed out" $result)
                    )
                    (ap "impossible happened" $result)
                )
            )
            (canon %init_peer_id% $result #result)
        )
    "#,
        <_>::default(),
        "#result.$[0]",
        1,
    )
    .await
    .unwrap();

    assert_eq!(&slow_result[0], "timed out");
}

#[tokio::test]
async fn debug_stringify() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    async fn stringify(value: impl Into<JValue>, client: &mut ConnectedClient) -> String {
        let mut result = exec_script_with(
            client,
            r#"(call relay ("debug" "stringify") [value] result)"#,
            hashmap! {
                "value" => value.into()
            },
            "result",
        )
        .await
        .unwrap();

        result[0].take().as_str().unwrap().to_string()
    }

    assert_eq!(stringify("hello", &mut client).await, r#""hello""#);
    assert_eq!(stringify(101, &mut client).await, r#"101"#);
    assert_eq!(
        stringify(json!({ "a": "b" }), &mut client).await,
        r#"{"a":"b"}"#
    );
    assert_eq!(stringify(json!(["a"]), &mut client).await, r#"["a"]"#);
    assert_eq!(
        stringify(json!(["a", "b"]), &mut client).await,
        r#"["a","b"]"#
    );

    let result = exec_script_with(
        &mut client,
        r#"(call relay ("debug" "stringify") [] result)"#,
        <_>::default(),
        "result",
    )
    .await
    .unwrap();
    assert_eq!(
        result[0].as_str().unwrap().to_string(),
        r#""<empty argument list>""#
    );

    let result = exec_script_with(
        &mut client,
        r#"(call relay ("debug" "stringify") ["a" "b"] result)"#,
        <_>::default(),
        "result",
    )
    .await
    .unwrap();
    assert_eq!(result[0].as_str().unwrap().to_string(), r#"["a","b"]"#);
}

#[tokio::test]
// checks that type errors are caught by XOR
async fn xor_type_error() {
    let result = exec_script(
        r#"
        (xor
            (call relay ("dist" "make_module_config") [obj obj obj])
            (call relay ("op" "identity") [%last_error%] error)
        )
        "#,
        hashmap! {
            "obj" => json!({"never valid": "ever"}),
        },
        "error",
        1,
    )
    .await
    .unwrap();
    assert_eq!(
        result[0].get("error_code"),
        Some(JValue::Number(10000.into())).as_ref()
    )
}

#[tokio::test]
async fn math_add() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "add", 2, 2, &mut client).await.unwrap(),
        json!(4)
    );
    assert!(format!(
        "{:?}",
        binary_with("math", "add", i64::MAX, i64::MAX, &mut client)
            .await
            .err()
            .unwrap()
    )
    .contains("overflow"));
}

#[tokio::test]
async fn math_sub() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "sub", 2, 2, &mut client).await.unwrap(),
        json!(0)
    );
    assert_eq!(
        binary_with("math", "sub", 2, 3, &mut client).await.unwrap(),
        json!(-1)
    );
}

#[tokio::test]
async fn math_mul() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "mul", 2, 2, &mut client).await.unwrap(),
        json!(4)
    );
    assert_eq!(
        binary_with("math", "mul", 2, 0, &mut client).await.unwrap(),
        json!(0)
    );
    assert_eq!(
        binary_with("math", "mul", 2, -1, &mut client)
            .await
            .unwrap(),
        json!(-2)
    );
}

#[tokio::test]
async fn math_fmul() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "fmul", 10, 0.66, &mut client)
            .await
            .unwrap(),
        json!(6)
    );
    assert_eq!(
        binary_with("math", "fmul", 0.5, 0.5, &mut client)
            .await
            .unwrap(),
        json!(0)
    );
    assert_eq!(
        binary_with("math", "fmul", 100.5, 0.5, &mut client)
            .await
            .unwrap(),
        json!(50)
    );
}

#[tokio::test]
async fn math_div() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "div", 2, 2, &mut client).await.unwrap(),
        json!(1)
    );
    assert_eq!(
        binary_with("math", "div", 2, 3, &mut client).await.unwrap(),
        json!(0)
    );
    assert_eq!(
        binary_with("math", "div", 10, 5, &mut client)
            .await
            .unwrap(),
        json!(2)
    );
    assert!(format!(
        "{:?}",
        binary_with("math", "div", 2, 0, &mut client)
            .await
            .err()
            .unwrap()
    )
    .contains("overflow"));
}

#[tokio::test]
async fn math_rem() {
    assert_eq!(binary("math", "rem", 10, 3).await.unwrap(), json!(1));
}

#[tokio::test]
async fn math_pow() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "pow", 2, 2, &mut client).await.unwrap(),
        json!(4)
    );
    assert_eq!(
        binary_with("math", "pow", 2, 0, &mut client).await.unwrap(),
        json!(1)
    );
}

#[tokio::test]
async fn math_log() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("math", "log", 2, 2, &mut client).await.unwrap(),
        json!(1)
    );
    assert_eq!(
        binary_with("math", "log", 2, 4, &mut client).await.unwrap(),
        json!(2)
    );
}

#[tokio::test]
async fn cmp_gt() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("cmp", "gt", 2, 4, &mut client).await.unwrap(),
        json!(false)
    );
    assert_eq!(
        binary_with("cmp", "gte", 2, 4, &mut client).await.unwrap(),
        json!(false)
    );
    assert_eq!(
        binary_with("cmp", "gte", 4, 2, &mut client).await.unwrap(),
        json!(true)
    );
    assert_eq!(
        binary_with("cmp", "gte", 2, 2, &mut client).await.unwrap(),
        json!(true)
    );
}

#[tokio::test]
async fn cmp_le() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("cmp", "lt", 2, 4, &mut client).await.unwrap(),
        json!(true)
    );
    assert_eq!(
        binary_with("cmp", "lte", 2, 4, &mut client).await.unwrap(),
        json!(true)
    );
    assert_eq!(
        binary_with("cmp", "lte", 4, 2, &mut client).await.unwrap(),
        json!(false)
    );
    assert_eq!(
        binary_with("cmp", "lte", 2, 2, &mut client).await.unwrap(),
        json!(true)
    );
}

#[tokio::test]
async fn cmp_cmp() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    assert_eq!(
        binary_with("cmp", "cmp", 2, 4, &mut client).await.unwrap(),
        json!(-1)
    );
    assert_eq!(
        binary_with("cmp", "cmp", 2, -4, &mut client).await.unwrap(),
        json!(1)
    );
    assert_eq!(
        binary_with("cmp", "cmp", 2, 2, &mut client).await.unwrap(),
        json!(0)
    );
}

#[tokio::test]
async fn array_sum() {
    assert_eq!(
        unary("array", "sum", vec![1, 2, 3]).await.unwrap(),
        json!(6)
    );
}

#[tokio::test]
async fn array_dedup() {
    match unary("array", "dedup", vec!["a", "a", "b", "c", "a", "b", "c"]).await {
        Ok(JValue::Array(arr)) => {
            let mut arr: Vec<_> = arr
                .into_iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            arr.sort();
            assert_eq!(arr, vec!["a", "b", "c"]);
        }
        unexpected => panic!("expected array, got {:?}", unexpected),
    };
}

#[tokio::test]
async fn array_intersect() {
    match binary(
        "array",
        "intersect",
        vec!["a", "b", "c"],
        vec!["c", "b", "d"],
    )
    .await
    {
        Ok(JValue::Array(arr)) => {
            let mut arr: Vec<_> = arr
                .into_iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            arr.sort();
            assert_eq!(arr, vec!["b", "c"])
        }
        unexpected => panic!("expected array, got {:?}", unexpected),
    };
}

#[tokio::test]
async fn array_diff() {
    match binary("array", "diff", vec!["a", "b", "c"], vec!["c", "b", "d"]).await {
        Ok(JValue::Array(arr)) => {
            assert_eq!(arr, vec!["a"])
        }
        unexpected => panic!("expected array, got {:?}", unexpected),
    }
}

#[tokio::test]
async fn array_sdiff() {
    match binary("array", "sdiff", vec!["a", "b", "c"], vec!["c", "b", "d"]).await {
        Ok(JValue::Array(arr)) => {
            let mut arr: Vec<_> = arr
                .into_iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            arr.sort();
            assert_eq!(arr, vec!["a", "d"])
        }
        unexpected => panic!("expected array, got {:?}", unexpected),
    }
}

#[tokio::test]
// checks that it is possible to use math's results as array indexes
async fn index_by_math() {
    let element = exec_script(
        r#"
    (seq
        (call relay ("math" "add") [x y] idx)
        (ap array.$[idx] element)
    )
    "#,
        hashmap! {
            "x" => json!(1),
            "y" => json!(2),
            "array" => json!(vec![1, 2, 3, 4, 5])
        },
        "element",
        1,
    )
    .await
    .unwrap();

    assert_eq!(element[0], json!(4));
}

#[tokio::test]
async fn service_mem() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (call relay ("stat" "service_memory") [service] memory_stat)
            (call %init_peer_id% ("op" "return") [memory_stat])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    use serde_json::Value::Array;

    if let [Array(stats)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(stats[0].get("name"), Some(&json!("tetraplets")));
    } else {
        panic!("incorrect args: expected single arrays of module memory stats")
    }
}

#[tokio::test]
async fn service_stats() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    let particle_id = client
        .send_particle(
            r#"
            (seq
                (seq
                    (call relay (service "not") [true] result)
                    (seq
                        (call relay (service "store") [key bigstring])
                        (call relay (service "delete") [key])
                    )
                )
                (call %init_peer_id% ("op" "return") [])
            )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "key" => json!("keeeyyy"),
                "bigstring" => json!("a".repeat(100_000)),
            },
        )
        .await;
    client
        .wait_particle_args(particle_id)
        .await
        .expect("receive particle");

    client
        .send_particle(
            r#"
        (seq
            (call relay ("stat" "service_stat") [service] stat)
            (call %init_peer_id% ("op" "return") [stat])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    if let Ok([result]) = client.receive_args().await.as_deref() {
        assert_eq!(result.get("error"), Some(&json!("")));
        assert_eq!(result.get("status"), Some(&json!(true)));

        assert_eq!(
            result.pointer("/result/0/total_stats/success_req_count"),
            Some(&json!(3))
        );

        let function_stats = result
            .pointer("/result/0/functions_stats")
            .unwrap()
            .as_array()
            .unwrap();
        let get_func = |name| {
            function_stats
                .iter()
                .find(|v| v.get("name") == Some(&json!(name)))
                .unwrap_or_else(|| panic!("'{name}' function not found"))
        };

        let not = get_func("not");
        assert_eq!(not.pointer("/stats/success_req_count"), Some(&json!(1)));
        assert_eq!(
            not.pointer("/stats/memory_deltas_bytes/total"),
            Some(&json!(0.0))
        );

        let store = get_func("store");
        assert_eq!(store.pointer("/stats/success_req_count"), Some(&json!(1)));
        assert_eq!(
            store.pointer("/stats/memory_deltas_bytes/total"),
            Some(&json!(65536.0))
        );

        let delete = get_func("delete");
        assert_eq!(delete.pointer("/stats/success_req_count"), Some(&json!(1)));
        assert_eq!(
            delete.pointer("/stats/memory_deltas_bytes/total"),
            Some(&json!(0.0))
        );
    } else {
        panic!("incorrect args: expected single arrays of module memory stats")
    }
}

#[tokio::test]
async fn service_stats_uninitialized() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"
        (seq
            (call relay ("stat" "service_stat") [service] stat)
            (call %init_peer_id% ("op" "return") [stat])
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    use serde_json::Value::Object;

    if let Ok([Object(result)]) = client.receive_args().await.as_deref() {
        assert_eq!(
            result.get("error"),
            Some(&json!(format!(
                "No stats were collected for the `{}` service",
                tetraplets_service.id
            )))
        );
        assert_eq!(result.get("status"), Some(&json!(false)));
    } else {
        panic!("incorrect args: expected single arrays of module memory stats")
    }
}

// Since KeyPair isn't in use for builtins anymore, we can't use this check as it is
// TODO: Need to ask Loysha to fix this test properly
/*
#[tokio::test]
async fn sign_verify() {
    let kp = KeyPair::generate_ed25519();
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.disabled_system_services
            .retain(|service| service != "registry");
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let _particle_id = client.send_particle(
        r#"
            (seq
                (seq
                    (call relay ("registry" "get_record_metadata_bytes") ["key_id" "" 0 "" "" [] [] []] data)
                    (seq
                        (call relay ("sig" "sign") [data] sig_result)
                        (xor
                            (call relay ("sig" "verify") [sig_result.$.signature.[0]! data] result)
                            (call %init_peer_id% ("op" "return") [sig_result.$.error])
                        )
                    )
                )
                (call %init_peer_id% ("op" "return") [data sig_result result])
            )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
        },
    );

    use serde_json::Value::Array;
    use serde_json::Value::Bool;
    use serde_json::Value::Object;

    if let [Array(data), Object(sig_result), Bool(result)] =
        client.receive_args().await.unwrap().as_slice()
    {
        let data: Vec<_> = data.iter().map(|n| n.as_u64().unwrap() as u8).collect();

        assert!(
            sig_result["success"].as_bool().unwrap(),
            "result isn't success"
        );
        let signature = sig_result["signature"].as_array().unwrap()[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u8)
            .collect();
        let signature = Signature::from_bytes(kp.public().get_key_format(), signature);
        assert!(result, "sig.verify result is false");
        assert!(
            kp.public().verify(&data, &signature).is_ok(),
            "kp verify failed"
        );
    } else {
        panic!("incorrect args: expected three arguments")
    }
}
*/

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sign_invalid_tetraplets_heavy() {
    enable_logs();
    let swarms = make_swarms_with_cfg(2, |mut cfg| {
        cfg.enabled_system_services = vec!["registry".to_string()];
        cfg
    })
    .await;

    let _span = tracing::info_span!("test", context = "test").entered();

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let relay = client.node.to_string();
    let wrong_peer = swarms[1].peer_id.to_base58();
    let result = client.execute_particle(
        r#"
            (seq
                (seq
                    (seq
                        (seq
                            (call relay ("op" "noop") [])
                            (call wrong_peer ("registry" "get_record_metadata_bytes") ["key_id" "" 0 "" "" [] [] []] data1)
                        )
                        (xor
                            (call relay ("sig" "sign") [data1] sig_result1)
                            (ap %last_error%.$.message host_error)
                        )
                    )
                    (seq
                        (seq
                            (call relay ("op" "identity") [array] data2)
                            (xor
                                (call relay ("sig" "sign") [data2] sig_result2)
                                (ap %last_error%.$.message srv_error)
                            )
                        )
                        (seq
                            (call relay ("registry" "get_key_bytes") ["label" [] 1 [] ""] data3)
                            (xor
                                (call relay ("sig" "sign") [data3] sig_result3)
                                (ap %last_error%.$.message func_error)
                            )
                        )
                    )
                )
                (call %init_peer_id% ("op" "return") [host_error srv_error func_error])
            )
        "#,
        hashmap! {
            "relay" => json!(relay),
            "wrong_peer" => json!(wrong_peer),
            "array" => json!(vec![1u8, 2u8, 3u8])
        },
    ).await;

    use serde_json::Value::String;

    if let [String(host_error), String(srv_error), String(func_error)] = result.unwrap().as_slice()
    {
        assert!(host_error.contains(&format!("data is expected to be produced by service 'registry' on peer '{relay}', was from peer '{wrong_peer}'")));
        assert!(srv_error.contains("data is expected to result from a call to 'registry.get_record_bytes' or 'registry.get_record_metadata_bytes', was from 'op.identity'"));
        assert!(func_error.contains("data is expected to result from a call to 'registry.get_record_bytes' or 'registry.get_record_metadata_bytes', was from 'registry.get_key_bytes'"));
    } else {
        panic!("incorrect args: expected three arguments")
    }
}

#[tokio::test]
async fn sig_verify_invalid_signature() {
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.enabled_system_services = vec!["registry".to_string()];
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

    client.send_particle(
        r#"
            (seq
                (seq
                    (call relay ("registry" "get_record_metadata_bytes") ["key_id" "" 0 "" "" [] [] []] data)
                    (seq
                        (call relay ("sig" "sign") [data] sig_result)
                        (seq
                            (call relay ("sig" "verify") [invalid_signature data] result1)
                            (call relay ("sig" "verify") [sig_result.$.signature.[0]! invalid_data] result2)
                        )
                    )
                )
                (call %init_peer_id% ("op" "return") [result1 result2])
            )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "invalid_signature" => json!(vec![1u8, 2u8, 3u8]),
            "invalid_data" => json!(vec![3u8, 2u8, 1u8])
        },
    ).await;

    use serde_json::Value::Bool;

    if let [Bool(result1), Bool(result2)] = client.receive_args().await.unwrap().as_slice() {
        assert!(
            !result1,
            "verification of invalid signature should be failed"
        );
        assert!(
            !result2,
            "signature verification of different data should be failed"
        );
    } else {
        panic!("incorrect args: expected three arguments")
    }
}

#[tokio::test]
async fn json_builtins() {
    let result = exec_script(
        r#"
        (seq
            (seq
                ;; create
                (seq
                    (call relay ("json" "obj") ["name" "nested_first" "num" 1] nested_first)
                    (call relay ("json" "obj") ["name" "nested_second" "num" 2] nested_second)
                )
                (seq
                    (call relay ("json" "obj") ["name" "outer_first" "num" 0 "nested" nested_first] outer_first)
                    (seq
                        (seq
                            (seq
                                (new $single-pair
                                    (seq
                                        (seq
                                            (ap "name" $single-pair)
                                            (ap "outer_first" $single-pair)
                                        )
                                        (seq
                                            (canon relay $single-pair #single-pair-1)
                                            (ap #single-pair-1 $pairs)
                                        )
                                    )
                                )
                                (new $single-pair
                                    (seq
                                        (seq
                                            (ap "num" $single-pair)
                                            (ap 0 $single-pair)
                                        )
                                        (seq
                                            (canon relay $single-pair #single-pair-2)
                                            (ap #single-pair-2 $pairs)
                                        )
                                    )
                                )
                            )
                            (seq
                                (new $single-pair
                                    (seq
                                        (seq
                                            (ap "nested" $single-pair)
                                            (ap nested_first $single-pair)
                                        )
                                        (seq
                                            (canon relay $single-pair #single-pair-3)
                                            (ap #single-pair-3 $pairs)
                                        )
                                    )
                                )
                                (canon relay $pairs #pairs)
                            )
                        )
                        (call relay ("json" "obj_pairs") [#pairs] outer_first_pairs)
                    )
                )
            )
            (seq
                ;; modify
                (seq
                    (seq
                        (call relay ("json" "put") [outer_first "nested" nested_second] outer_tmp_second)
                        (call relay ("json" "puts") [outer_tmp_second "name" "outer_second" "num" 3] outer_second)
                    )
                    (new $puts-pairs
                        (seq
                            (seq
                                (new $single-pair
                                    (seq
                                        (seq
                                            (ap "name" $single-pair)
                                            (ap "outer_second" $single-pair)
                                        )
                                        (seq
                                            (canon relay $single-pair #single-pair-4)
                                            (ap #single-pair-4 $puts-pairs)
                                        )
                                    )
                                )
                                (new $single-pair
                                    (seq
                                        (seq
                                            (ap "num" $single-pair)
                                            (ap 3 $single-pair)
                                        )
                                        (seq
                                            (canon relay $single-pair #single-pair-5)
                                            (ap #single-pair-5 $puts-pairs)
                                        )
                                    )
                                )
                            )
                            (seq
                                (canon relay $puts-pairs #puts-pairs)
                                (call relay ("json" "puts_pairs") [outer_tmp_second #puts-pairs] outer_second_pairs)
                            )
                        )
                    )
                )
                ;; stringify and parse
                (seq
                    (call relay ("json" "stringify") [outer_first] outer_first_string)
                    (call relay ("json" "parse") [outer_first_string] outer_first_parsed)
                )
            )
        )
    "#,
        hashmap! {},
        r"nested_first nested_second outer_first outer_second outer_first_string outer_first_parsed outer_first_pairs outer_second_pairs",
        1,
    ).await.expect("execute script");

    if let [nested_first, nested_second, outer_first, outer_second, outer_first_string, outer_first_parsed, outer_first_pairs, outer_second_pairs] =
        result.as_slice()
    {
        let nf_expected = json!({"name": "nested_first", "num": 1});
        let ns_expected = json!({"name": "nested_second", "num": 2});

        let of_expected = json!({"name": "outer_first", "nested": nf_expected, "num": 0 });
        let os_expected = json!({"name": "outer_second", "num": 3, "nested": ns_expected });

        assert_eq!(&nf_expected, nested_first);
        assert_eq!(&ns_expected, nested_second);
        assert_eq!(&of_expected, outer_first);
        assert_eq!(&of_expected, outer_first_pairs);
        assert_eq!(&os_expected, outer_second);
        assert_eq!(&os_expected, outer_second_pairs);
        assert_eq!(&of_expected.to_string(), outer_first_string);
        assert_eq!(&of_expected, outer_first_parsed);
    } else {
        panic!("Result is of incorrect shape: {:?}", result);
    }
}

async fn binary(
    service: &str,
    func: &str,
    x: impl Into<JValue>,
    y: impl Into<JValue>,
) -> Result<JValue, Report> {
    let result = exec_script(
        r#"(call relay (service func) [x y] result)"#,
        hashmap! {
            "service" => service.into(),
            "func" => func.into(),
            "x" => x.into(),
            "y" => y.into()
        },
        "result",
        1,
    )
    .await;

    result.map(|mut r| r[0].take())
}

async fn binary_with(
    service: &str,
    func: &str,
    x: impl Into<JValue>,
    y: impl Into<JValue>,
    client: &mut ConnectedClient,
) -> Result<JValue, Report> {
    let result = exec_script_with(
        client,
        r#"(call relay (service func) [x y] result)"#,
        hashmap! {
            "service" => service.into(),
            "func" => func.into(),
            "x" => x.into(),
            "y" => y.into()
        },
        "result",
    )
    .await;

    result.map(|mut r| r[0].take())
}

async fn unary(service: &str, func: &str, x: impl Into<JValue>) -> Result<JValue, Report> {
    let result = exec_script(
        r#"(call relay (service func) [x] result)"#,
        hashmap! {
            "service" => service.into(),
            "func" => func.into(),
            "x" => x.into(),
        },
        "result",
        1,
    )
    .await;

    result.map(|mut r| r[0].take())
}

async fn exec_script(
    script: &str,
    args: HashMap<&'static str, JValue>,
    result: &str,
    node_count: usize,
) -> Result<Vec<JValue>, Report> {
    let swarms = make_swarms(node_count).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    let res = exec_script_with(&mut client, script, args, result).await;
    swarms
        .into_iter()
        .map(|s| s.exit_outlet.send(()))
        .for_each(drop);
    res
}

async fn exec_script_with<'a>(
    client: &mut ConnectedClient,
    script: &'a str,
    mut args: HashMap<&'static str, JValue>,
    result: &'a str,
) -> Result<Vec<JValue>, Report> {
    args.insert("relay", json!(client.node.to_string()));

    client
        .execute_particle(
            f!(r#"
        (seq
            {script}
            (call %init_peer_id% ("op" "return") [{result}])
        )
        "#),
            args,
        )
        .await
}

#[tokio::test]
async fn add_alias_list() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    let alias = "tetraplets".to_string();
    client
        .send_particle(
            r#"
        (seq
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call relay ("srv" "list") [] list)
            )
            (call %init_peer_id% ("op" "return") [list])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!(alias.clone())
            },
        )
        .await;

    use serde_json::Value::Array;

    if let [Array(list)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(list.len(), 1);

        let service_info = &list[0];
        let aliases = service_info
            .as_object()
            .unwrap()
            .get("aliases")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].as_str().unwrap(), alias)
    } else {
        panic!("incorrect args: expected an array")
    }
}

#[tokio::test]
async fn aliases_restart() {
    let kp = KeyPair::generate_ed25519();
    let swarms = make_swarms_with_keypair(1, kp.clone()).await;
    let tmp_dir = swarms[0].tmp_dir.clone();

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    let alias = "tetraplets".to_string();
    client
        .send_particle(
            r#"
        (xor
            (seq
                (call relay ("srv" "add_alias") [alias service])
                (call %init_peer_id% ("op" "return") ["ok"])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
                "alias" => json!(alias.clone())
            },
        )
        .await;

    if let [JValue::String(result)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(*result, "ok");
    }

    use serde_json::Value::Array;

    // stop swarm
    swarms
        .into_iter()
        .map(|s| s.exit_outlet.send(()))
        .for_each(drop);

    let swarms = make_swarms_with_cfg(1, move |mut cfg| {
        cfg.keypair = kp.clone();
        cfg.tmp_dir = tmp_dir.clone();
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

    client
        .send_particle(
            r#"
        (seq
            (call relay ("srv" "list") [] list_after)
            (call %init_peer_id% ("op" "return") [list_after])
        )
    "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    if let [Array(after)] = client.receive_args().await.unwrap().as_slice() {
        assert_eq!(after.len(), 1);
        let service_info = &after[0];
        let aliases = service_info
            .as_object()
            .unwrap()
            .get("aliases")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].as_str().unwrap(), alias)
    } else {
        panic!("incorrect args: expected array")
    }
}

#[ignore]
#[tokio::test]
async fn subnet_resolve() {
    let worker1 = chain_connector::Deal::Worker {
        offchainId: peer_id_to_bytes(RandomPeerId::random()).into(),
        peerId: peer_id_to_bytes(RandomPeerId::random()).into(),
        provider: Default::default(),
        joinedEpoch: Default::default(),
        onchainId: Default::default(),
        computeUnitIds: vec![hex!(
            "0000000000000000000000000000000000000000000000000000000000000001"
        )
        .into()],
    };

    let worker2 = chain_connector::Deal::Worker {
        offchainId: Default::default(),
        peerId: peer_id_to_bytes(RandomPeerId::random()).into(),
        provider: Default::default(),
        joinedEpoch: Default::default(),
        onchainId: Default::default(),
        computeUnitIds: vec![hex!(
            "0000000000000000000000000000000000000000000000000000000000000002"
        )
        .into()],
    };
    let resolve_result = encode_hex_0x(Array::<chain_connector::Deal::Worker>::abi_encode(&vec![
        worker1.clone(),
        worker2.clone(),
    ]));

    // Create a mock
    let mut server = mockito::Server::new_async().await;
    let url = server.url();
    let mock = server
        .mock("POST", "/")
        .with_body_from_request(move |req| {
            let body = req.body().expect("mock: get req body");
            let body: serde_json::Value =
                serde_json::from_slice(body).expect("mock: parse req body as json");
            assert!(body.is_object());
            assert_eq!(
                body.as_object().unwrap()["method"].as_str().unwrap(),
                "eth_call"
            );

            let id = body.as_object().unwrap()["id"].as_u64().unwrap();

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": resolve_result,
            })
            .to_string()
            .into()
        })
        .expect(2)
        .with_status(200)
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

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("subnet" "resolve") ["9DcaFca9B88f49d91c38a32E7d9A86a7d9a37B04"] subnet1)
                (seq
                    (call relay ("subnet" "resolve") ["0x9DcaFca9B88f49d91c38a32E7d9A86a7d9a37B04"] subnet2)
                    (call relay ("subnet" "resolve") ["invalid_deal_id"] invalid)
                )
            )
            (call %init_peer_id% ("op" "return") [subnet1 subnet2 invalid])
        )
    "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
        },
    ).await;

    let mut result = client.receive_args().await.unwrap();

    let subnet: SubnetResolveResult = serde_json::from_value(result.remove(0)).unwrap();
    let subnet_second: SubnetResolveResult = serde_json::from_value(result.remove(0)).unwrap();
    let invalid: SubnetResolveResult = serde_json::from_value(result.remove(0)).unwrap();

    assert!(!invalid.success, "invalid should be failed");
    assert_eq!(invalid.error.len(), 1);
    assert_eq!(
        invalid.error[0],
        "Invalid deal id 'invalid_deal_id': invalid length"
    );

    assert_eq!(subnet, subnet_second);

    assert!(subnet.success, "{:?}", subnet.error);
    assert_eq!(subnet.error.len(), 0);
    let workers: Vec<_> = subnet
        .workers
        .iter()
        .map(|p| (p.cu_ids.clone(), p.host_id.clone(), p.worker_id.clone()))
        .collect();

    assert_eq!(
        workers,
        vec![
            (
                vec![encode_hex_0x(worker1.computeUnitIds[0].0).to_string()],
                parse_peer_id(&worker1.peerId.0).unwrap().to_base58(),
                vec![parse_peer_id(&worker1.offchainId.0).unwrap().to_base58()],
            ),
            (
                vec![encode_hex_0x(worker2.computeUnitIds[0].0).to_string()],
                parse_peer_id(&worker2.peerId.0).unwrap().to_base58(),
                vec![]
            )
        ]
    );

    // check that mock was called
    mock.assert();
}
