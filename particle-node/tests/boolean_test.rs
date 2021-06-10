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

use std::thread::sleep;

use fstrings::f;
use maplit::hashmap;
use serde_json::json;

use eyre::WrapErr;
use services_utils::load_module;
use test_utils::{create_service, make_swarms, ConnectedClient, KAD_TIMEOUT};

#[test]
fn pass_boolean() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets"),
    );

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

    client.send_particle(script, data.clone());

    let args = client.receive_args().wrap_err("receive").unwrap();
    assert_eq!(args, vec![json!(false), json!(true)]);
}
