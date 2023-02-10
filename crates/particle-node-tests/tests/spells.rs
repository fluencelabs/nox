/*
 * Copyright 2021 Fluence Labs Limited
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
#![feature(assert_matches)]
use std::assert_matches::assert_matches;
use std::collections::HashMap;
use std::time::Duration;

use eyre::Context;
use maplit::hashmap;
use serde_json::{json, Value as JValue};

use connected_client::ConnectedClient;
use created_swarm::{make_swarms, make_swarms_with_builtins};
use fluence_spell_dtos::trigger_config::TriggerConfig;
use log_utils::enable_logs;
use service_modules::load_module;
use spell_event_bus::api::{TriggerInfo, TriggerInfoAqua, MAX_PERIOD_SEC};
use test_utils::create_service;

type SpellId = String;
type ScopePeerId = String;

fn create_spell(
    client: &mut ConnectedClient,
    script: &str,
    config: TriggerConfig,
    init_data: JValue,
) -> (SpellId, ScopePeerId) {
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => init_data,
    };
    client.send_particle(
        r#"
        (seq
            (call relay ("spell" "install") [script data config] spell_id)
            (seq
                (call relay ("scope" "get_peer_id") [] scope_peer_id)
                (call client ("return" "") [spell_id scope_peer_id])
            )
        )"#,
        data.clone(),
    );

    let response = client.receive_args().wrap_err("receive").unwrap();
    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);
    let scope_peer_id = response[1].as_str().unwrap().to_string();
    assert_ne!(scope_peer_id.len(), 0);

    (spell_id, scope_peer_id)
}

#[test]
fn spell_simple_test() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (seq
                (seq
                    (call %init_peer_id% (spell_id "get_script_source_from_file") [] script)
                    (call %init_peer_id% (spell_id "get_u32") ["counter"] counter)
                )
                (call "{}" ("return" "") [script.$.source_code counter])
            )
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 0;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    let response = client.receive_args().wrap_err("receive").unwrap();
    let result = response[0].as_str().unwrap().to_string();
    assert!(response[1]["success"].as_bool().unwrap());
    let counter = response[1]["num"].as_u64().unwrap();

    assert_eq!(result, script);
    assert_ne!(counter, 0);
}

#[test]
fn spell_error_handling_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let failing_script = r#"
        (xor
            (call %init_peer_id% ("srv" "remove") ["non_existent_srv_id"])
            (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])        
        )"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 1;
    config.clock.start_sec = 1;

    let (spell_id, worker_id) = create_spell(&mut client, failing_script, config, json!({}));

    // let's retrieve error from the first spell particle
    let particle_id = format!("spell_{}_{}", spell_id, 0);
    let mut result = vec![];
    for _ in 1..10 {
        let data = hashmap! {
            "spell_id" => json!(spell_id),
            "particle_id" => json!(particle_id),
            "client" => json!(client.peer_id.to_string()),
            "worker" => json!(worker_id),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_errors") [particle_id] result)
            )
            (call client ("return" "") [result])
        )"#,
            data.clone(),
        );

        let response = client.receive_args().wrap_err("receive").unwrap();
        if !response[0].as_array().unwrap().is_empty() {
            result = response[0].as_array().unwrap().clone();
        }
    }

    assert_eq!(result.len(), 1);
}

#[test]
fn spell_args_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("getDataSrv" "key") [] value)
            )
            (call "{}" ("return" "") [value])
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 1;
    config.clock.start_sec = 1;
    let expected_value = json! ({"a": "b", "c": 1});
    create_spell(
        &mut client,
        &script,
        config,
        json!({ "key": expected_value }),
    );

    let response = client.receive_args().wrap_err("receive").unwrap();
    assert_eq!(response[0], expected_value);
}

#[test]
fn spell_return_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("json" "obj") ["key" "value"] obj)
            )
            (seq
                (call %init_peer_id% ("callbackSrv" "response") [obj])
                (seq
                    (seq
                        (call %init_peer_id% (spell_id "get_string") ["key"] value_raw)
                        (call %init_peer_id% ("json" "parse") [value_raw.$.str] value)
                    )    
                    (call "{}" ("return" "") [value])
                )
            )
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 1;
    config.clock.start_sec = 1;

    create_spell(&mut client, &script, config, json!({}));

    let response = client.receive_args().wrap_err("receive").unwrap();
    let value = response[0].as_str().unwrap().to_string();

    assert_eq!(value, "value");
}

// Check that oneshot spells are actually executed and executed only once
#[test]
fn spell_run_oneshot() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (call %init_peer_id% (spell_id "set_string") ["result" "done"])
        )"#;

    // Note that when period is 0, the spell is executed only once
    let mut config = TriggerConfig::default();
    config.clock.start_sec = 1;
    let (spell_id, worker_id) = create_spell(&mut client, script, config, json!({}));

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_u32") ["counter"] counter)
            )
            (call client ("return" "") [counter])
        )"#,
        data.clone(),
    );

    std::thread::sleep(Duration::from_millis(100));
    let response = client.receive_args().wrap_err("receive").unwrap();
    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 1);
    }
}

// The config considered empty if start_sec is 0. In this case we don't schedule a spell.
// Script installation will fail because no triggers configured.
#[test]
fn spell_install_ok_empty_config() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;

    // Note that when period is 0, the spell is executed only once
    let config = TriggerConfig::default();
    let (spell_id, worker_id) = create_spell(&mut client, script, config, json!({}));

    // The spell should be installed, but should not be subscribed to any triggers
    // We cannot truly check that the spell isn't subscribed to anything right now, but we can check that
    // it's counter is zero on different occasions:

    // 1. Check that the spell wasn't executed immediately after installation (the case of `start_sec` <= now)
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_u32") ["counter"] counter)
            )
            (call %init_peer_id% ("return" "") [counter])
        )"#,
        hashmap! {
            "worker" => json!(worker_id),
            "spell_id" => json!(spell_id),
            "relay" => json!(client.node.to_string()),
        },
    );
    let response = client
        .receive_args()
        .wrap_err("receive counter first try")
        .unwrap();
    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 0);
    }
    // 2. Connect and disconnect a client to the same node. The spell should not be executed
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    drop(connected);

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_u32") ["counter"] counter)
            )
            (call %init_peer_id% ("return" "") [counter])
        )"#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "worker" => json!(worker_id),
            "spell_id" => json!(spell_id),
        },
    );
    let response = client
        .receive_args()
        .wrap_err("receive counter second try")
        .unwrap();
    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 0);
    }

    // 3. We cannot check that it's not scheduled to run in the future, but it's ok for now.
}

#[test]
fn spell_install_fail_large_period() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    // Note that when period is 0, the spell is executed only once
    let mut config = TriggerConfig::default();
    config.clock.period_sec = MAX_PERIOD_SEC + 1;
    config.clock.start_sec = 1;

    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };
    client.send_particle(
        r#"
        (xor
            (call relay ("spell" "install") [script data config] spell_id)
            (call client ("return" "") [%last_error%.$.message])
        )"#,
        data,
    );

    if let [JValue::String(error_msg)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let msg = "Local service error, ret_code is 1, error message is '\"Error: invalid config: period is too big.";
        assert!(error_msg.starts_with(msg));
    }
}

// Also the config considered invalid if the end_sec is in the past.
// In this case we don't schedule a spell and return error.
#[test]
fn spell_install_fail_end_sec_past() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    // Note that when period is 0, the spell is executed only once
    let mut config = TriggerConfig::default();
    config.clock.start_sec = 10;
    config.clock.end_sec = 1;

    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };
    client.send_particle(
        r#"
        (xor
            (call relay ("spell" "install") [script data config] spell_id)
            (call client ("return" "") [%last_error%.$.message])
        )"#,
        data.clone(),
    );

    if let [JValue::String(error_msg)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let expected = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past";
        assert!(
            error_msg.starts_with(expected),
            "expected:\n{expected}\ngot:\n{error_msg}"
        );
    }
}

// Also the config considered invalid if the end_sec is less than start_sec.
// In this case we don't schedule a spell and return error.
#[test]
fn spell_install_fail_end_sec_before_start() {
    enable_logs();

    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Note that when period is 0, the spell is executed only once
    let mut config = TriggerConfig::default();
    config.clock.start_sec = now as u32 + 100;
    config.clock.end_sec = now as u32 + 90;

    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };
    client.send_particle(
        r#"
        (xor
            (call relay ("spell" "install") [script data config] spell_id)
            (call client ("return" "") [%last_error%.$.message])
        )"#,
        data.clone(),
    );

    if let [JValue::String(error_msg)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let expected = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past";
        assert!(
            error_msg.starts_with(expected),
            "expected:\n{expected}\ngot:\n{error_msg}"
        );
    }
}

#[test]
fn spell_store_trigger_config() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let mut config = TriggerConfig::default();
    config.clock.period_sec = 13;
    config.clock.start_sec = 10;
    let (spell_id, worker_id) = create_spell(&mut client, script, config.clone(), json!({}));
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_trigger_config") [] config)
            )
            (call client ("return" "") [config])
        )"#,
        data.clone(),
    );

    let response = client.receive_args().wrap_err("receive").unwrap();
    if response[0]["success"].as_bool().unwrap() {
        let result_config = serde_json::from_value(response[0]["config"].clone()).unwrap();
        assert_eq!(config, result_config);
    }
}

#[test]
fn spell_remove() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    let (spell_id, scope) = create_spell(&mut client, script, config, json!({}));

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };

    client.send_particle(
        r#"
    (seq
        (call relay ("spell" "list") [] list)
        (call client ("return" "") [list])
    )"#,
        data.clone(),
    );

    if let [JValue::String(result_spell_id)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(&spell_id, result_spell_id);
    }

    client.send_particle(
        r#"
        (seq
            (seq
                (call relay ("scope" "get_peer_id") [] scope_id)
                (call relay ("spell" "remove") [spell_id])
            )
            (seq
                (call relay ("spell" "list") [] list)
                (call client ("return" "") [list])
            )
        )
        "#,
        data.clone(),
    );

    if let [JValue::Array(created_spells)] = client
        .receive_args()
        .wrap_err(format!("receive by {}, scope {}", client.peer_id, scope))
        .unwrap()
        .as_slice()
    {
        assert!(created_spells.is_empty(), "no spells should exist");
    }
}

#[test]
fn spell_remove_by_alias() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (seq
                (seq
                    (seq
                        (call %init_peer_id% ("srv" "add_alias") ["alias" spell_id])
                        (call %init_peer_id% ("spell" "list") [] before)
                    )
                    (call %init_peer_id% ("spell" "remove") ["alias"])
                )
                (seq
                    (call %init_peer_id% ("spell" "list") [] after)
                    (call "{}" ("return" "") [before after])
                )
            )
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    let (spell_id, _) = create_spell(&mut client, &script, config, json!({}));

    if let [JValue::Array(before), JValue::Array(after)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(before.len(), 1);
        assert_eq!(before[0], spell_id);
        assert!(after.is_empty());
    }
}

#[test]
fn spell_remove_spell_as_service() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    let (spell_id, _) = create_spell(&mut client, script, config, json!({}));

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };

    client.send_particle(
        r#"
        (xor
            (call relay ("srv" "remove") [spell_id])
            (call client ("return" "") [%last_error%.$.message])
        )
        "#,
        data.clone(),
    );

    if let [JValue::String(msg)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let expected = "cannot call function 'remove_service': cannot remove a spell";
        assert!(
            msg.contains(expected),
            "should contain `{expected}`, given msg `{msg}`"
        );
    }
}

#[test]
fn spell_remove_service_as_spell() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let service = create_service(
        &mut client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share").expect("load module"),
    );

    let data = hashmap! {
        "service_id" => json!(service.id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };

    client.send_particle(
        r#"
        (xor
            (call relay ("spell" "remove") [service_id])
            (call client ("return" "") [%last_error%.$.message])
        )
        "#,
        data.clone(),
    );

    if let [JValue::String(msg)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let expected = "cannot call function 'remove_spell': the service isn't a spell";
        assert!(
            msg.contains(expected),
            "should contain `{expected}`, given msg `{msg}`"
        );
    }
}

#[test]
fn spell_call_by_alias() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (seq
                (seq
                    (call %init_peer_id% ("srv" "add_alias") ["alias" spell_id])
                    (call %init_peer_id% ("alias" "get_u32") ["counter"] counter)
                )

                (call "{}" ("return" "") [counter])
            )
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    if let [JValue::Number(counter)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_ne!(counter.as_i64().unwrap(), 0);
    }
}

#[test]
fn spell_trigger_connection_pool() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% (spell_id "get_u32") ["counter"] counter)
            )
            (call "{}" ("return" "") [spell_id])
        )
    "#,
        client.peer_id
    );
    let mut config = TriggerConfig::default();
    config.connections.connect = true;
    let (spell_id1, _) = create_spell(&mut client, &script, config, json!({}));

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    let (spell_id2, _) = create_spell(&mut client, &script, config, json!({}));

    // This connect should trigger the spell
    let connect_num = 5;
    for _ in 0..connect_num {
        ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();
    }

    let mut spell1_counter = 0;
    let mut spell2_counter = 0;

    // we must receive `connect_num` messages from spell1 subscribed on connect and `connect_num` messages
    // from spell1 subscribed on disconnect, so 2 * `connect_num` messages in total
    for _ in 0..2 * connect_num {
        if let [spell_reply] = client
            .receive_args()
            .wrap_err("receive")
            .unwrap()
            .as_slice()
        {
            let spell_id = spell_reply.as_str().unwrap();
            assert!(
                spell_id == spell_id1 || spell_id == spell_id2,
                "spell id must be one of the subscribed ones"
            );

            if spell_id == spell_id1 {
                spell1_counter += 1;
            } else {
                spell2_counter += 1;
            }
        }
    }

    assert_eq!(
        spell1_counter, connect_num,
        "spell subscribed on connect must be triggered {connect_num} times"
    );
    assert_eq!(
        spell2_counter, connect_num,
        "spell subscribed on disconnect must be triggered {connect_num} times"
    );
}

#[test]
fn spell_timer_trigger_mailbox_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% (spell_id "list_pop_string") ["trigger_mailbox"] trigger)
            )
            (seq
                (call %init_peer_id% ("json" "parse") [trigger.$.str] obj)
                (call "{}" ("return" "") [obj])
            )
        )
    "#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 0;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    let value = client.receive_args().wrap_err("receive").unwrap()[0]
        .as_object()
        .cloned()
        .unwrap();

    assert!(value.contains_key("peer"));
    assert!(value.contains_key("timer"));
    assert_eq!(value["peer"].as_array().unwrap().len(), 0);
    let timer_opt = value["timer"].as_array().cloned().unwrap();
    assert_eq!(timer_opt.len(), 1);
    let timer = timer_opt[0].as_object().cloned().unwrap();
    assert!(timer.contains_key("timestamp"));
    assert!(timer["timestamp"].as_i64().unwrap() > 0);
}

#[test]
fn spell_connection_pool_trigger_mailbox_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% (spell_id "list_pop_string") ["trigger_mailbox"] trigger)
            )
            (seq
                (call %init_peer_id% ("json" "parse") [trigger.$.str] obj)
                (call "{}" ("return" "") [obj])
            )
        )
    "#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    create_spell(&mut client, &script, config.clone(), json!({}));

    let disconnected_client = ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();
    let disconnected_client_peer_id = disconnected_client.peer_id;
    disconnected_client.client.stop();

    let value = client.receive_args().wrap_err("receive").unwrap()[0]
        .as_object()
        .cloned()
        .unwrap();

    assert!(value.contains_key("peer"));
    assert!(value.contains_key("timer"));
    assert_eq!(value["timer"].as_array().unwrap().len(), 0);
    let peer_opt = value["peer"].as_array().cloned().unwrap();
    assert_eq!(peer_opt.len(), 1);
    let peer = peer_opt[0].as_object().cloned().unwrap();
    assert!(peer.contains_key("peer_id"));
    assert!(peer.contains_key("connected"));
    assert_eq!(
        peer["peer_id"].as_str().unwrap(),
        disconnected_client_peer_id.to_base58()
    );
    assert!(!peer["connected"].as_bool().unwrap());
}

#[test]
fn spell_set_u32() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(r#"(call "{}" ("return" "") ["called"])"#, client.peer_id);
    let mut config = TriggerConfig::default();
    config.connections.connect = true;

    let (spell_id, worker_id) = create_spell(&mut client, &script, config.clone(), json!({}));

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    client.send_particle(
        r#"(seq
            (seq
                (seq
                    (call relay ("op" "noop") [])
                    (call worker (spell_id "get_u32") ["test"] absent)
                )
                (seq
                    (call worker (spell_id "set_u32") ["test" 1])
                    (seq
                        (call worker (spell_id "get_u32") ["test"] one)
                        (seq
                            (call worker (spell_id "set_u32") ["test" 2])
                            (call worker (spell_id "get_u32") ["test"] two)
                        )
                    )
                )
            )
            (call %init_peer_id% ("return" "") [absent one two])
           )"#,
        data,
    );
    let mut result = client.receive_args().wrap_err("receive").unwrap();
    assert_eq!(result.len(), 3);
    let (absent, one, two) = (result.remove(0), result.remove(0), result.remove(0));

    assert_eq!(absent["absent"], json!(true));

    assert_eq!(one["absent"], json!(false));
    assert_eq!(one["num"], json!(1));

    assert_eq!(two["absent"], json!(false));
    assert_eq!(two["num"], json!(2));
}

#[test]
fn spell_peer_id_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
            (call "{}" ("return" "") [%init_peer_id%])
        "#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.start_sec = 1;
    let (_, scope_peer_id) = create_spell(&mut client, &script, config, json!({}));

    let response = client.receive_args().wrap_err("receive").unwrap();

    let result = response[0].as_str().unwrap().to_string();

    assert_eq!(result, scope_peer_id);
}
#[test]
fn spell_update_config() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"(seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% (spell_id "list_pop_string") ["trigger_mailbox"] result)
             )
            (call "{}" ("return" "") [result])
        )"#,
        client.peer_id
    );
    let mut config = TriggerConfig::default();
    config.connections.connect = true;
    let (spell_id, _) = create_spell(&mut client, &script, config, json!({}));
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();

    if let [JValue::Object(x)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(x["absent"], JValue::Bool(false), "spell must be triggered");
        let info: TriggerInfoAqua = serde_json::from_str(x["str"].as_str().unwrap()).unwrap();
        let info: TriggerInfo = info.into();
        assert_matches!(info, TriggerInfo::Peer(p) if p.connected, "spell must be triggered by the `connected` event");
    } else {
        panic!("wrong result from spell, expected trigger info with the `connected` event");
    }

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    client.send_particle(
        r#"(seq
            (call relay ("spell" "update_trigger_config") [spell_id config])
            (call %init_peer_id% ("return" "") ["done"])
           )"#,
        data,
    );
    let result = client.receive_args().wrap_err("receive").unwrap().pop();
    let result = match result {
        Some(JValue::String(result)) => result,
        None => panic!("no results from update_trigger_config particle"),
        other => panic!(
            "expected JSON String from update_trigger_config particle, got {:?}",
            other
        ),
    };
    assert_eq!(result, "done", "spell must be updated");

    drop(connected);

    if let [JValue::Object(x)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(x["absent"], JValue::Bool(false), "spell must be triggered");
        let info: TriggerInfoAqua = serde_json::from_str(x["str"].as_str().unwrap()).unwrap();
        let info: TriggerInfo = info.into();
        assert_matches!(info, TriggerInfo::Peer(p) if !p.connected, "spell must be triggered by the `disconnected` event");
    } else {
        panic!("wrong result from spell, expect trigger info with the `disconnected` event");
    }
}

#[test]
fn spell_update_config_stopped_spell() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"(seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% (spell_id "list_pop_string") ["trigger_mailbox"] result)
             )
            (call "{}" ("return" "") [result])
        )"#,
        client.peer_id
    );
    // create periodic spell
    let config = TriggerConfig::default();
    let (spell_id, _) = create_spell(&mut client, &script, config, json!({}));

    // Update trigger config to do something.
    let mut config = TriggerConfig::default();
    config.clock.start_sec = 1;
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    client.send_particle(
        r#"(seq
            (call relay ("spell" "update_trigger_config") [spell_id config])
            (call %init_peer_id% ("return" "") ["done"])
           )"#,
        data,
    );
    let result = client.receive_args().wrap_err("receive").unwrap().pop();
    let result = match result {
        Some(JValue::String(result)) => result,
        None => panic!("no results from update_trigger_config particle"),
        other => panic!(
            "expected JSON String from update_trigger_config particle, got {:?}",
            other
        ),
    };
    assert_eq!(result, "done", "spell must be updated");

    if let [JValue::Object(x)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(x["absent"], JValue::Bool(false), "spell must be triggered");
        let info: TriggerInfoAqua = serde_json::from_str(x["str"].as_str().unwrap()).unwrap();
        let info: TriggerInfo = info.into();
        assert_matches!(
            info,
            TriggerInfo::Timer(_),
            "spell must be triggered by the timer event"
        );
    } else {
        panic!("wrong result from spell, expect trigger info with the timer event");
    }
}

#[test]
fn resolve_alias_wrong_worker() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (xor
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (seq
                    (call %init_peer_id% ("srv" "add_alias") ["alias" spell_id])
                    (seq
                        (call %init_peer_id% ("srv" "resolve_alias") ["alias"] resolved)
                        (seq
                            (call "{0}" ("srv" "resolve_alias") ["alias"] not_resolved)
                            (call "{1}" ("return" "") ["test failed"])
                        )
                    )
                )
             )
            (call "{1}" ("return" "") [%last_error%.$.message])
        )"#,
        client.node, client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    if let [JValue::String(error)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert!(error.starts_with("Local service error, ret_code is 1, error message is '\"Error: Service with alias 'alias' is not found on worker"));
    }
}

#[test]
fn resolve_global_alias() {
    let swarms = make_swarms(1);

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    );

    client.send_particle(
        r#"(call relay ("srv" "add_alias") ["alias" service])"#,
        hashmap! {
            "relay" => json!(client.node.to_string()),
            "service" => json!(tetraplets_service.id),
        },
    );

    let script = format!(
        r#"
        (seq
            (call %init_peer_id% ("srv" "resolve_alias") ["alias"] resolved)
            (call "{0}" ("return" "") [resolved])
        )"#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    if let [JValue::String(resolved)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*resolved, tetraplets_service.id);
    }
}

#[test]
fn worker_sig_test() {
    let swarms = make_swarms_with_builtins(
        1,
        "tests/builtins/services".as_ref(),
        None,
        None,
    );

    let mut client = ConnectedClient::connect_to(
        swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("registry" "get_record_bytes") ["key_id" "" [] [] 1 []] data)
                (seq
                    (call %init_peer_id% ("sig" "sign") [data] sig_result)
                    (call %init_peer_id% ("sig" "verify") [sig_result.$.signature.[0]! data] result)
                )
            )
            (call "{0}" ("op" "return") [sig_result result])
        )
       "#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    create_spell(&mut client, &script, config, json!({}));

    use serde_json::Value::Bool;
    use serde_json::Value::Object;

    if let [Object(sig_result), Bool(result)] =
        client.receive_args().unwrap().as_slice()
    {
        assert!(sig_result["success"].as_bool().unwrap());
        assert!(result);
    } else {
        panic!("incorrect args: expected two arguments")
    }
}
