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
use test_utils::{
    create_greeting_service, create_service, load_module, make_swarms, ConnectedClient, KAD_TIMEOUT,
};

#[test]
fn test_tetraplets() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client1 =
        ConnectedClient::connect_to_with_peer_id(swarms[0].1.clone(), Some(swarms[0].3.clone()))
            .wrap_err("connect client")
            .unwrap();
    let tetraplets_service = create_service(
        &mut client1,
        "tetraplets",
        load_module(
            "tests/tetraplets/target/wasm32-wasi/release",
            "tetraplets.wasm",
        ),
    );

    let mut client2 = ConnectedClient::connect_to(swarms[1].1.clone())
        .wrap_err("connect client")
        .unwrap();
    let greeting_service = create_greeting_service(&mut client2);

    let script = f!(r#"
    (xor
        (seq
            (seq
                (call "{client2.node}" ("op" "identity") [])
                (seq
                    (call "{client2.node}" (greeting_service_id "greeting") [my_name] result)
                    (call "{client1.node}" (tetraplets_service_id "get_tetraplets") [result] tetraplets)
                )
            )
            (seq
                (call "{client2.node}" ("op" "identity") [])
                (call "{client2.peer_id}" ("return" "") [tetraplets])
            )
        )
        (seq
            (call "{client2.node}" ("op" "identity") [])
            (call "{client2.peer_id}" ("return" "") ["XOR: get_tetraplets() failed"])
        )
    )"#);

    let data = hashmap! {
        "host" => json!(client1.node.to_string()),
        "relay" => json!(client2.node.to_string()),
        "client" => json!(client2.peer_id.to_string()),
        "tetraplets_service_id" => json!(tetraplets_service.id),
        "greeting_service_id" => json!(greeting_service.id),
        "my_name" => json!("justprosh"),
    };

    client2.send_particle(script, data.clone());

    let args = client2.receive_args().wrap_err("receive").unwrap();
    let mut args = args.into_iter();
    let tetraplets = args.next().unwrap();
    let tetraplets: Vec<Vec<SecurityTetraplet>> = serde_json::from_value(tetraplets)
        .wrap_err("deserialize tetraplets")
        .unwrap();
    assert_eq!(tetraplets.len(), 1);
    assert_eq!(tetraplets[0].len(), 1);

    let tetraplet = &tetraplets[0][0];
    assert_eq!(tetraplet.function_name, "greeting");
    assert_eq!(tetraplet.peer_pk, client2.peer_id.to_base58());

    assert_eq!(tetraplet.service_id, greeting_service.id.as_str());
}
