/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
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
        swarms[0].network_key.clone()
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
