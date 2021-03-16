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

use test_utils::{make_swarms, ConnectedClient};

use eyre::WrapErr;
use fstrings::f;
use maplit::hashmap;
use serde_json::json;

#[macro_use]
extern crate fstrings;

#[test]
fn stream_hello() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    client.send_particle(
        r#"
        (call relay ("script" "add") [script "0"])
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "script" => json!(script),
        },
    );

    for _ in 1..10 {
        let res = client.receive_args().wrap_err("receive").unwrap();
        let res = res.into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[test]
fn remove_script() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to_with_peer_id(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    client.send_particle(
        r#"
        (seq
            (call relay ("script" "add") [script "0"] id)
            (call client ("op" "return") [id])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "script" => json!(script),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    let script_id = args.into_iter().next().unwrap();
    let remove_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "remove") [id] removed)
            (call client ("op" "return") [removed])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "id" => json!(script_id),
        },
    );

    let removed = client.wait_particle_args(remove_id).unwrap();
    assert_eq!(removed, vec![serde_json::Value::Bool(true)]);

    let list_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "list") [] list)
            (call client ("op" "return") [list])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );
    let list = client.wait_particle_args(list_id).unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[test]
/// Check that auto-particle can be delivered through network hops
fn script_routing() {
    let swarms = make_swarms(3);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

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
            (call second ("script" "add") [script "0"] id)
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "second" => json!(swarms[1].peer_id.to_string()),
            "script" => json!(script),
        },
    );

    for _ in 1..10 {
        let res = client.receive_args().wrap_err("receive args").unwrap();
        let res = res.into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[test]
fn autoremove_singleshot() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    client.send_particle(
        r#"
        (call relay ("script" "add") [script])
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "script" => json!(script),
        },
    );

    let res = client.receive_args().wrap_err("receive args").unwrap();
    let res = res.into_iter().next().unwrap();
    assert_eq!(res, "hello");

    let list_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "list") [] list)
            (call client ("op" "return") [list])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );
    let list = client.wait_particle_args(list_id).unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[test]
fn autoremove_failed() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        INVALID SCRIPT
    "#);

    client.send_particle(
        r#"
        (call relay ("script" "add") [script])
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "script" => json!(script),
        },
    );

    for _ in 0..500 {
        let list_id = client.send_particle(
            r#"
            (seq
                (call relay ("script" "list") [] list)
                (call client ("op" "return") [list])
            )
            "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        );
        let list = client.wait_particle_args(list_id).unwrap();
        if list == vec![serde_json::Value::Array(vec![])] {
            return;
        }
    }

    panic!("failed script wasn't deleted in time or at all");
}

#[test]
fn remove_script_unauth() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to_with_peer_id(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    // add script
    client.send_particle(
        r#"
        (seq
            (call relay ("script" "add") [script "0"] id)
            (call client ("op" "return") [id])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "script" => json!(script),
        },
    );

    let args = client.receive_args().wrap_err("receive args").unwrap();
    let script_id = args.into_iter().next().unwrap();

    // try to remove from another client, should fail
    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let remove_id = client2.send_particle(
        r#"
        (xor
            (call relay ("script" "remove") [id] removed)
            (call client ("op" "return") ["failed"])
        )
        "#,
        hashmap! {
            "relay" => json!(client2.node.to_string()),
            "client" => json!(client2.peer_id.to_string()),
            "id" => json!(script_id),
        },
    );

    let removed = client2.wait_particle_args(remove_id).unwrap();
    assert_eq!(
        removed,
        vec![serde_json::Value::String("failed".to_string())]
    );

    // check script is still in the list
    let list_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "list") [] list)
            (call client ("op" "return") [list])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );
    let list = client
        .wait_particle_args(list_id)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    if let serde_json::Value::Array(list) = list {
        assert_eq!(list.len(), 1);
    } else {
        panic!("expected array");
    }

    // remove with owner
    let remove_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "remove") [id] removed)
            (call client ("op" "return") [removed])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
            "id" => json!(script_id),
        },
    );

    // check removal succeeded
    let removed = client.wait_particle_args(remove_id).unwrap();
    assert_eq!(removed, vec![serde_json::Value::Bool(true)]);

    // check script is not in the list anymore
    let list_id = client.send_particle(
        r#"
        (seq
            (call relay ("script" "list") [] list)
            (call client ("op" "return") [list])
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "client" => json!(client.peer_id.to_string()),
        },
    );
    let list = client.wait_particle_args(list_id).unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}
