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

use eyre::WrapErr;
use fstrings::f;
use maplit::hashmap;
use serde_json::json;

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use log_utils::enable_logs;
use service_modules::load_module;
use test_utils::create_service;

#[tokio::test]
async fn pass_boolean() {
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

    let script = f!(r#"
    (seq
        (seq
            (call host (service_id "not") [tru] not_true)
            (call host (service_id "not") [fal] not_false)
        )
        (call %init_peer_id% ("op" "return") [not_true not_false])
    )"#);

    let data = hashmap! {
        "host" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "service_id" => json!(tetraplets_service.id),
        "tru" => json!(true),
        "fal" => json!(false),
    };

    let args = client.execute_particle(script, data.clone()).await.unwrap();

    assert_eq!(args, vec![json!(false), json!(true)]);
}
