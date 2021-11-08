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
use serde_json::Value as JValue;

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use humantime_serde::re::humantime::format_duration;
use now_millis::now;
use std::time::Duration;

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

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
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
            (call "{client.node}" ("op" "noop") [])
            (call "{client.peer_id}" ("op" "return") ["hello"])
        )
    "#);

    client.send_particle(
        r#"
        (seq
            (call relay ("op" "noop") [])
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

    let mut get_list = move || {
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
        list
    };

    let timeout = Duration::from_secs(5);
    let deadline = now() + timeout;

    // wait for script to appear in the list
    while now() < deadline {
        let list = get_list();

        if list.len() == 0 {
            continue;
        }

        if let JValue::Array(arr) = &list[0] {
            if arr.len() == 0 {
                continue;
            }
            let failures = arr[0].get("failures");
            assert_eq!(failures, Some(&json!(0)));
            break;
        } else {
            panic!("expected array");
        }
    }

    if now() >= deadline {
        panic!("timed out adding script after {}", format_duration(timeout));
    }

    // wait for script to disappear from the list
    while now() < deadline {
        let list = get_list();
        if list == vec![serde_json::Value::Array(vec![])] {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    panic!(
        "failed script wasn't deleted after {}",
        format_duration(timeout)
    );
}

#[test]
fn remove_script_unauth() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
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

#[test]
fn remove_script_management_key() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
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

    // try to remove with management key
    let mut client2 = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let remove_id = client2.send_particle(
        r#"
        (seq
            (call relay ("script" "remove") [id] removed)
            (call client ("op" "return") [removed])
        )
        "#,
        hashmap! {
            "relay" => json!(client2.node.to_string()),
            "client" => json!(client2.peer_id.to_string()),
            "id" => json!(script_id),
        },
    );

    // check removal succeeded
    let removed = client2.wait_particle_args(remove_id).unwrap();
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

#[test]
fn add_script_delay() {
    let swarms = make_swarms(1);

    let delay = 3u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let now_id = client.send_particle(
        r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] now)
            (seq
                (call relay ("script" "add") [script "0" delay])
                (call %init_peer_id% ("op" "return") [now])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "delay" => json!(delay),
            "script" => json!(script),
        },
    );

    let res = client.wait_particle_args(now_id).unwrap().pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 3u64;
    let expected = now + delay;
    let check_range = expected - eps..expected + eps;
    assert!(check_range.contains(&res));
}

#[test]
fn add_script_delay_oneshot() {
    let swarms = make_swarms(1);

    let delay = 4u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let now_id = client.send_particle(
        r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] now)
            (seq
                (call relay ("script" "add") [script $none delay])
                (call %init_peer_id% ("op" "return") [now])
            )
        )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "delay" => json!(delay),
            "script" => json!(script),
        },
    );

    let res = client.wait_particle_args(now_id).unwrap().pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 10u64;
    let expected = now + delay;
    let check_range = expected - eps..expected + eps;
    assert!(check_range.contains(&res));

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
fn add_script_random_delay() {
    let swarms = make_swarms(1);

    let interval = 3u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let now_id = client.send_particle(
        f!(r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] now)
            (seq
                (call relay ("script" "add") [script "{interval}"])
                (call %init_peer_id% ("op" "return") [now])
            )
        )
        "#),
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "script" => json!(script),
        },
    );

    let res = client.wait_particle_args(now_id).unwrap().pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 2u64;
    let expected = now + interval + eps;
    assert!((now..=expected).contains(&res));
}
