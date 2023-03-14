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
use serde_json::Value as JValue;
use serde_json::{json, Value};

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use humantime_serde::re::humantime::format_duration;
use log_utils::enable_logs;
use now_millis::now;
use service_modules::load_module;
use std::time::Duration;
use test_utils::{create_service, CreatedService};

#[tokio::test]
async fn stream_hello() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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
        let res = client.receive_args().await.wrap_err("receive").unwrap();
        let res = res.into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[tokio::test]
async fn remove_script() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    let args = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let script_id = args.into_iter().next().unwrap();
    let removed = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    assert_eq!(removed, vec![serde_json::Value::Bool(true)]);

    let list = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[tokio::test]
/// Check that auto-particle can be delivered through network hops
async fn script_routing() {
    let swarms = make_swarms(3).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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
        let res = client
            .receive_args()
            .await
            .wrap_err("receive args")
            .unwrap();
        let res = res.into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[tokio::test]
async fn autoremove_singleshot() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let res = client
        .receive_args()
        .await
        .wrap_err("receive args")
        .unwrap();
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
    let list = client.wait_particle_args(list_id).await.unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

async fn get_list(client: &mut ConnectedClient) -> Vec<Value> {
    let list_id = client.send_particle(
        r#"
            (seq
                (call relay ("script" "list") [] list)
                (call client ("op" "return") [list])
            )
            "#,
        hashmap! {
        "relay" => json ! (client.node.to_string()),
        "client" => json ! (client.peer_id.to_string()),
        },
    );

    client.wait_particle_args(list_id).await.unwrap()
}

#[tokio::test]
async fn autoremove_failed() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let timeout = Duration::from_secs(5);
    let deadline = now() + timeout;

    // wait for script to appear in the list
    while now() < deadline {
        let list = get_list(&mut client).await;

        if list.is_empty() {
            continue;
        }

        if let JValue::Array(arr) = &list[0] {
            if arr.is_empty() {
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
        let list = get_list(&mut client).await;
        if list == vec![serde_json::Value::Array(vec![])] {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    panic!(
        "failed script wasn't deleted after {}",
        format_duration(timeout)
    );
}

#[tokio::test]
async fn remove_script_unauth() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    // add script
    let args = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let script_id = args.into_iter().next().unwrap();

    // try to remove from another client, should fail
    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    let removed = client2
        .execute_particle(
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
        )
        .await
        .unwrap();

    assert_eq!(
        removed,
        vec![serde_json::Value::String("failed".to_string())]
    );

    // check script is still in the list
    let list = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    let list = list.into_iter().next().unwrap();
    if let serde_json::Value::Array(list) = list {
        assert_eq!(list.len(), 1);
    } else {
        panic!("expected array");
    }

    // remove with owner
    let removed = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    // check removal succeeded
    assert_eq!(removed, vec![serde_json::Value::Bool(true)]);

    // check script is not in the list anymore
    let list = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[tokio::test]
async fn remove_script_management_key() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);

    // add script
    let args = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let script_id = args.into_iter().next().unwrap();

    // try to remove with management key
    let mut client2 = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
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
    let removed = client2.wait_particle_args(remove_id).await.unwrap();
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
    let list = client.wait_particle_args(list_id).await.unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[tokio::test]
async fn add_script_delay() {
    let swarms = make_swarms(1).await;

    let delay = 3u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let mut res = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let res = res.pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().await.wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 10u64;
    let expected = now + delay;
    let check_range = expected - eps..expected + eps;
    assert!(check_range.contains(&res));
}

#[tokio::test]
async fn add_script_delay_oneshot() {
    let swarms = make_swarms(1).await;

    let delay = 4u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let mut res = client
        .execute_particle(
            r#"
        (seq
            (call relay ("peer" "timestamp_sec") [] now)
            (seq
                (call relay ("script" "add") [script [] delay])
                (call %init_peer_id% ("op" "return") [now])
            )
        )
        "#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "delay" => json!(delay),
                "script" => json!(script),
            },
        )
        .await
        .unwrap();

    let res = res.pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().await.wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 10u64;
    let expected = now + delay;
    let check_range = expected - eps..expected + eps;
    assert!(check_range.contains(&res));

    let list = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    assert_eq!(list, vec![serde_json::Value::Array(vec![])]);
}

#[tokio::test]
async fn add_script_random_delay() {
    enable_logs();
    let swarms = make_swarms(1).await;

    let interval = 3u64;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (seq
            (call "{client.node}" ("peer" "timestamp_sec") [] result)
            (call "{client.peer_id}" ("op" "return") [result])
        )
    "#);

    let mut res = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    let res = res.pop().unwrap();
    let now = res.as_u64().unwrap();

    let res = client.receive_args().await.wrap_err("receive").unwrap();
    let res = res.into_iter().next().unwrap().as_u64().unwrap();
    let eps = 2u64;
    let expected = now + interval + eps;
    log::info!("res {}", res);
    log::info!("expected {}", expected);
    assert!((now..=expected).contains(&res));
}

async fn create_file_share(client: &mut ConnectedClient) -> CreatedService {
    create_service(
        client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share").expect("load module"),
    )
    .await
}

#[tokio::test]
async fn add_script_from_vault_ok() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);
    let fileshare = create_file_share(&mut client).await;
    client.send_particle(
        r#"
            (seq
                (call relay (fileshare "create_vault_file_path") [script] filepath)
                (call relay ("script" "add_from_vault") [filepath "0"])
            )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "script" => json!(script),
            "fileshare" => json!(fileshare.id),
        },
    );

    for _ in 1..10 {
        let res = client.receive_args().await.wrap_err("receive").unwrap();
        let res = res.into_iter().next().unwrap();
        assert_eq!(res, "hello");
    }
}

#[tokio::test]
async fn add_script_from_vault_wrong_vault() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = f!(r#"
        (call "{client.peer_id}" ("op" "return") ["hello"])
    "#);
    let fileshare = create_file_share(&mut client).await;
    let result = client.execute_particle(
        r#"
           (xor
               (call relay ("script" "add_from_vault") ["/tmp/vault/another-particle-id/script" "0"])
               (call %init_peer_id% ("op" "return") [%last_error%.$.message])
           )
        "#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "script" => json!(script),
            "fileshare" => json!(fileshare.id),
        },
    ).await.unwrap();

    if let [JValue::String(error_msg)] = result.as_slice() {
        let expected_error_prefix = "Local service error, ret_code is 1, error message is '\"Error: Incorrect vault path `/tmp/vault/another-particle-id/script";
        assert!(
            error_msg.starts_with(expected_error_prefix),
            "expected:\n{expected_error_prefix}\ngot:\n{error_msg}"
        );
    }
}
