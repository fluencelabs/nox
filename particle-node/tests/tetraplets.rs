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
use fluence_app_service::SecurityTetraplet;
use test_utils::{create_service, load_module, make_swarms, ConnectedClient, KAD_TIMEOUT};

#[test]
fn test_tetraplets() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone())
        .wrap_err("connect client")
        .unwrap();
    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets.wasm"),
    );

    let script = f!(r#"
    (seq
        (seq
            (seq
                (call host ("op" "identity") ["test"] result)
                (call host (service_id "get_tetraplets") [result] first_tetraplets)
            )
            (seq
                (call host ("op" "identity") [])
                (call host (service_id "get_tetraplets") [first_tetraplets.$.[0][0].peer_pk] second_tetraplets)
            )
        )
        (seq
            (call host ("op" "identity") [])
            (call client ("return" "") [first_tetraplets second_tetraplets])
        )
    )"#);

    let data = hashmap! {
        "host" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "service_id" => json!(tetraplets_service.id),
    };

    client.send_particle(script, data.clone());

    let args = client.receive_args().wrap_err("receive").unwrap();
    let mut args = args.into_iter();
    let first_tetraplets = args.next().unwrap();
    let first_tetraplets: Vec<Vec<SecurityTetraplet>> = serde_json::from_value(first_tetraplets)
        .wrap_err("deserialize tetraplets")
        .unwrap();
    assert_eq!(first_tetraplets.len(), 1);
    assert_eq!(first_tetraplets[0].len(), 1);

    let tetraplet = &first_tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "identity");
    assert_eq!(tetraplet.peer_pk, client.node.to_base58());
    assert_eq!(tetraplet.json_path, "");
    assert_eq!(tetraplet.service_id, "op");

    let second_tetraplets = args.next().unwrap();
    let second_tetraplets: Vec<Vec<SecurityTetraplet>> = serde_json::from_value(second_tetraplets)
        .wrap_err("deserialize tetraplets")
        .unwrap();

    let tetraplet = &second_tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "get_tetraplets");
    assert_eq!(tetraplet.peer_pk, client.node.to_base58());
    assert_eq!(tetraplet.json_path, "$.[0][0].peer_pk");
    assert_eq!(tetraplet.service_id, tetraplets_service.id.as_str());
}
