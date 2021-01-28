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

use test_utils::{make_swarms, timeout, ConnectedClient};

use fstrings::f;
use maplit::hashmap;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

#[macro_use]
extern crate fstrings;

#[test]
fn stream_hello() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    client.send_particle(
        r#"
        (call relay ("script" "add") [script])
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "script" => json!(script),
        },
    );

    for _ in 1..10 {
        let res = client.receive_args().into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[test]
fn remove_script() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    client.send_particle(
        r#"
        (seq
            (call relay ("script" "add") [script] id)
            (call client ("op" "return") [id])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "script" => json!(script),
        },
    );

    let script_id = client.receive_args().into_iter().next().unwrap();
    client.send_particle(
        r#"
        (call relay ("script" "remove") [id])
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "id" => json!(script_id),
        },
    );

    async_std::task::block_on(timeout(
        Duration::from_secs(1),
        async_std::task::spawn(async move {
            loop {
                if client.maybe_receive().is_none() {
                    break;
                }
            }
        }),
    ))
    .expect("script wasn't deleted");
}

#[test]
/// Check that auto-particle can be delivered through network hops
fn script_routing() {
    let swarms = make_swarms(3);

    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let script = f!(r#"
        (seq
            (call "{client.node}" ("op" "identity") [])
            (call "{client.peer_id}" ("op" "return") ["hello"])
        )
    "#);

    client.send_particle(
        r#"
        (seq
            (call relay ("op" "identity") [])
            (call second ("script" "add") [script] id)
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "second" => json!(swarms[1].0.to_string()),
            "script" => json!(script),
        },
    );

    for _ in 1..10 {
        log::info!("waiting for hello");
        let res = client.receive_args().into_iter().next().unwrap();
        assert_eq!(res, "hello");
        log::info!("got hello");
    }
}
