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

use particle_protocol::Particle;
use test_utils::{create_greeting_service, make_swarms, ConnectedClient, KAD_TIMEOUT};

#[test]
fn create_service() {
    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client1 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let service = create_greeting_service(&mut client1);

    let mut client2 = ConnectedClient::connect_to(swarms[1].1.clone()).expect("connect client");

    let script = f!(r#"
        (seq
            (seq
                (call "{client2.node}" ("op" "identity") [])
                (call "{client1.node}" (service_id "greeting") [my_name] greeting)
            )
            (seq
                (call "{client2.node}" ("op" "identity") [])
                (call "{client2.peer_id}" ("return" "") [greeting])
            )
        )"#);
    client2.send_particle(
        script,
        hashmap! {
            "host" => json!(client1.node.to_string()),
            "relay" => json!(client2.node.to_string()),
            "client" => json!(client2.peer_id.to_string()),
            "service_id" => json!(service.id),
            "my_name" => json!("folex"),
        },
    );

    let response = client2.receive_args();
    assert_eq!(response[0].as_str().unwrap(), "Hi, folex")
}
