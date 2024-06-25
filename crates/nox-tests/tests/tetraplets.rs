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

use fstrings::f;
use maplit::hashmap;
use serde_json::json;

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use fluence_app_service::SecurityTetraplet;
use service_modules::load_module;
use test_utils::create_service;

use eyre::WrapErr;

#[tokio::test]
async fn test_tetraplets() {
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

    let script = f!(r#"
    (seq
        (seq
            (seq
                (seq
                    (ap "test" ap_literal)
                    (seq
                        (call host ("op" "identity") ["test"] result)
                        (ap result ap_result)
                    )
                )
                (seq
                    (seq
                        (call host (service_id "get_tetraplets") [ap_literal] ap_literal_tetraplets)
                        (call host (service_id "get_tetraplets") [result] first_tetraplets)
                    )
                    (call host (service_id "get_tetraplets") [ap_result] ap_first_tetraplets)
                )
            )
            (seq
                (call host ("op" "noop") [])
                (call host (service_id "get_tetraplets") [first_tetraplets.$.[0][0].peer_pk] second_tetraplets)
            )
        )
        (seq
            (call host ("op" "noop") [])
            (call client ("return" "") [ap_literal_tetraplets first_tetraplets ap_first_tetraplets second_tetraplets])
        )
    )"#);

    let data = hashmap! {
        "host" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "service_id" => json!(tetraplets_service.id),
    };

    client.send_particle(script, data.clone()).await;

    let args = client.receive_args().await.wrap_err("receive").unwrap();
    let mut args = args.into_iter();

    let ap_literal_tetraplets = args.next().unwrap();
    let ap_literal_tetraplets: Vec<Vec<SecurityTetraplet>> =
        serde_json::from_value(ap_literal_tetraplets)
            .wrap_err("deserialize tetraplets")
            .unwrap();
    assert_eq!(ap_literal_tetraplets.len(), 1);
    assert_eq!(ap_literal_tetraplets[0].len(), 1);
    let tetraplet = &ap_literal_tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "");
    assert_eq!(tetraplet.peer_pk, client.peer_id.to_base58());
    assert_eq!(tetraplet.lens, "");
    assert_eq!(tetraplet.service_id, "");

    let first_tetraplets = args.next().unwrap();
    let first_tetraplets: Vec<Vec<SecurityTetraplet>> = serde_json::from_value(first_tetraplets)
        .wrap_err("deserialize tetraplets")
        .unwrap();
    assert_eq!(first_tetraplets.len(), 1);
    assert_eq!(first_tetraplets[0].len(), 1);

    let tetraplet = &first_tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "identity");
    assert_eq!(tetraplet.peer_pk, client.node.to_base58());
    assert_eq!(tetraplet.lens, "");
    assert_eq!(tetraplet.service_id, "op");

    let ap_first_tetraplets = args.next().unwrap();
    let ap_first_tetraplets: Vec<Vec<SecurityTetraplet>> =
        serde_json::from_value(ap_first_tetraplets)
            .wrap_err("deserialize tetraplets")
            .unwrap();
    let ap_tetraplet = &ap_first_tetraplets[0][0];
    assert_eq!(tetraplet, ap_tetraplet);

    let second_tetraplets = args.next().unwrap();
    let second_tetraplets: Vec<Vec<SecurityTetraplet>> = serde_json::from_value(second_tetraplets)
        .wrap_err("deserialize tetraplets")
        .unwrap();

    let tetraplet = &second_tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "get_tetraplets");
    assert_eq!(tetraplet.peer_pk, client.node.to_base58());
    assert_eq!(tetraplet.lens, ".$.[0].[0].peer_pk");
    assert_eq!(tetraplet.service_id, tetraplets_service.id.as_str());
}
