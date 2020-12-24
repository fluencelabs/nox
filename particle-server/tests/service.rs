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
use test_utils::{create_greeting_service, enable_logs, make_swarms, ConnectedClient, KAD_TIMEOUT};

#[test]
fn create_service() {
    enable_logs();

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

#[test]
fn send_particle() {
    enable_logs();

    let swarms = make_swarms(3);
    sleep(KAD_TIMEOUT);

    let mut client1 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let particle = f!(r#"
{{"id":"bd3ec49c-6dfc-497c-a290-2e4b1308affd","init_peer_id":"{client1.peer_id}","timestamp":1608819120033,"ttl":30000,"script":"\n            (seq\n                (call \"12D3KooWEXNUbCXooUwHrHBbrmjsrpHXoEphPwbjQXEGyzbqKnE9\" (\"op\" \"identity\") [])\n                (call \"12D3KooWBT4PJ4DMAPgQciBPXmiM7daioaU1ChJVSgPN2b8qoJKj\" (\"return\" \"\") [\"yo\"])\n            )\n        ","signature":"Dv6EWi1NWjqtT7C78sKHn5HKPiNUNAjpFEbfsYVp7QuScddmD7LK7nSRifXygafXuiHCLfHq99YTh1VpCcT6soE","data":[91,123,34,99,97,108,108,34,58,123,34,114,101,113,117,101,115,116,95,115,101,110,116,34,58,34,49,50,68,51,75,111,111,87,66,84,52,80,74,52,68,77,65,80,103,81,99,105,66,80,88,109,105,77,55,100,97,105,111,97,85,49,67,104,74,86,83,103,80,78,50,98,56,113,111,74,75,106,34,125,125,93]}}
    "#);

    println!("particle: {:?}", particle);

    let particle: Particle = serde_json::from_str(particle.as_str()).unwrap();

    client1.send(particle);

    client1.receive_args();
}
