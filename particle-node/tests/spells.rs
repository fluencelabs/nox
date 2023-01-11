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
use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use eyre::Context;

use fluence_spell_dtos::trigger_config::TriggerConfig;
use log_utils::enable_logs;
use maplit::hashmap;
use serde_json::{json, Value as JValue};
use service_modules::load_module;
use spell_event_bus::api::MAX_PERIOD_SEC;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use test_utils::create_service;

fn create_spell(
    client: &mut ConnectedClient,
    script: &str,
    config: TriggerConfig,
    init_data: HashMap<String, String>,
) -> String {
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(init_data).to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (call relay ("spell" "install") [script data config] spell_id)
            (call client ("return" "") [spell_id])
        )"#,
        data.clone(),
    );

    let response = client.receive_args().wrap_err("receive").unwrap();
    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);

    spell_id
}

#[test]
fn spell_simple_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (seq
                (call %init_peer_id% (spell_id "get_script_source_from_file") [] script)
                (call %init_peer_id% (spell_id "set_string") ["result" script.$.source_code])
            )
        )"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 0;
    config.clock.start_sec = 1;
    let spell_id = create_spell(&mut client, script, config, hashmap! {});

    let mut result = "".to_string();
    let mut counter = 0;
    for _ in 1..10 {
        let data = hashmap! {
            "spell_id" => json!(spell_id),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
        (seq
            (seq
                (call relay (spell_id "get_string") ["result"] result)
                (call relay (spell_id "get_u32") ["counter"] counter)
            )
            (call client ("return" "") [result counter])
        )"#,
            data.clone(),
        );

        let response = client.receive_args().wrap_err("receive").unwrap();
        if response[0]["success"].as_bool().unwrap() && response[1]["success"].as_bool().unwrap() {
            result = response[0]["str"].as_str().unwrap().to_string();
            counter = response[1]["num"].as_u64().unwrap();
        }
    }

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

    let spell_id = create_spell(&mut client, failing_script, config, hashmap! {});

    // let's retrieve error from the first spell particle
    let particle_id = format!("spell_{}_{}", spell_id, 0);
    let mut result = vec![];
    for _ in 1..10 {
        let data = hashmap! {
            "spell_id" => json!(spell_id),
            "particle_id" => json!(particle_id),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
        (seq  
            (call relay (spell_id "get_errors") [particle_id] result)
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

    let script = r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("getDataSrv" "key") [] value)
            )
            (call %init_peer_id% (spell_id "set_string") ["result" value])
        )"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 1;
    config.clock.start_sec = 1;

    let spell_id = create_spell(
        &mut client,
        script,
        config,
        hashmap! {"key".to_string() => "value".to_string()},
    );

    let mut result = "".to_string();
    let mut value = "".to_string();
    for _ in 1..10 {
        let data = hashmap! {
            "spell_id" => json!(spell_id),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
        (seq
            (seq
                (call relay (spell_id "get_string") ["result"] result_raw)
                (call relay (spell_id "get_string") ["key"] value_raw)
            )
            (call client ("return" "") [result_raw value_raw])
        )"#,
            data.clone(),
        );

        let response = client.receive_args().wrap_err("receive").unwrap();
        if response[0]["success"].as_bool().unwrap() && response[1]["success"].as_bool().unwrap() {
            result = JValue::from_str(response[0]["str"].as_str().unwrap())
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            value = JValue::from_str(response[1]["str"].as_str().unwrap())
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
        }
    }

    assert_eq!(result, "value");
    assert_eq!(result, value);
}

#[test]
fn spell_return_test() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("json" "obj") ["key" "value"] obj)
            )
            (seq
                (call %init_peer_id% ("json" "stringify") [obj] result)
                (call %init_peer_id% ("callbackSrv" "response") [result])
            )
        )"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 1;
    config.clock.start_sec = 1;

    let spell_id = create_spell(&mut client, script, config, hashmap! {});

    let mut value = "".to_string();
    for _ in 1..10 {
        let data = hashmap! {
            "spell_id" => json!(spell_id),
            "client" => json!(client.peer_id.to_string()),
            "relay" => json!(client.node.to_string()),
        };
        client.send_particle(
            r#"
        (seq
            (call relay (spell_id "get_string") ["key"] value_raw)
            (call client ("return" "") [value_raw])
        )"#,
            data.clone(),
        );

        let response = client.receive_args().wrap_err("receive").unwrap();
        if response[0]["success"].as_bool().unwrap() {
            value = JValue::from_str(response[0]["str"].as_str().unwrap())
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
        }
    }

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
    let spell_id = create_spell(&mut client, script, config.clone(), hashmap! {});

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (call relay (spell_id "get_u32") ["counter"] counter)
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
fn spell_install_fail_empty_config() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    // Note that when period is 0, the spell is executed only once
    let config = TriggerConfig::default();

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
        let msg = "Local service error, ret_code is 1, error message is '\"Error: config is empty, nothing to do\"'";
        assert_eq!(msg, error_msg);
    }
}

#[test]
fn spell_install_fail_large_period() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
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

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
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
        let msg = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past\"'";
        assert!(error_msg.starts_with(msg));
    }
}

// Also the config considered invalid if the end_sec is less than start_sec.
// In this case we don't schedule a spell and return error.
#[test]
fn spell_install_fail_end_sec_before_start() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
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
        let msg = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past\"'";
        assert!(error_msg.starts_with(msg));
    }
}

#[test]
fn spell_store_trigger_config() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
    let mut config = TriggerConfig::default();
    config.clock.period_sec = 13;
    config.clock.start_sec = 10;
    let spell_id = create_spell(&mut client, script, config.clone(), hashmap! {});
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (call relay (spell_id "get_trigger_config") [] config)
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

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;
    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    let spell_id = create_spell(&mut client, script, config, hashmap! {});

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
            (call relay ("spell" "remove") [spell_id])
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
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert!(created_spells.is_empty(), "no spells should exist");
    }
}

#[test]
fn spell_remove_spell_as_service() {
    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "idenitfy") [] x)"#;

    let mut config = TriggerConfig::default();
    config.clock.period_sec = 2;
    config.clock.start_sec = 1;
    let spell_id = create_spell(&mut client, script, config, hashmap! {});

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
        let msg_end = "cannot call function 'remove_service': cannot remove a spell\"'";
        assert!(msg.ends_with(msg_end), "should end with `{}`", msg_end);
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
        let msg_end = "cannot call function 'remove_spell': the service isn't a spell\"'";
        assert!(msg.ends_with(msg_end), "should end with `{}`", msg_end);
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
            (seq
                (call %init_peer_id% ("json" "obj") ["spell_id" spell_id "counter" counter] obj)
                (call "{}" ("return" "") [obj])
            )
        )
    "#,
        client.peer_id
    );
    let mut config = TriggerConfig::default();
    config.connections.connect = true;
    let spell_id1 = create_spell(&mut client, &script, config.clone(), hashmap! {});

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    let spell_id2 = create_spell(&mut client, &script, config, hashmap! {});

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
            let is_ok = spell_reply["counter"]["success"].as_bool().unwrap();
            assert!(is_ok, "we must receive a success response");
            let counter = spell_reply["counter"]["num"].as_u64().unwrap();

            let spell_id = spell_reply["spell_id"].as_str().unwrap();
            assert!(
                spell_id == spell_id1 || spell_id == spell_id2,
                "spell id must be one of the subscribed ones"
            );

            if spell_id == spell_id1 {
                spell1_counter += 1;
                assert_eq!(
                    spell1_counter, counter,
                    "we should receive messages from spells in order"
                );
            } else {
                spell2_counter += 1;
                assert_eq!(
                    spell2_counter, counter,
                    "we should receive messages from spells in order"
                );
            }
        }
    }

    assert_eq!(
        spell1_counter, connect_num,
        "spell subscribed on connect must be triggered {} times",
        connect_num
    );
    assert_eq!(
        spell2_counter, connect_num,
        "spell subscribed on disconnect must be triggered {} times",
        connect_num
    );
}

#[test]
fn spell_update_config() {
    enable_logs();

    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(r#"(call "{}" ("return" "") ["called"])"#, client.peer_id);
    let mut config = TriggerConfig::default();
    config.connections.connect = true;
    let spell_id = create_spell(&mut client, &script, config.clone(), hashmap! {});

    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();

    if let [JValue::String(x)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(x, "called", "spell must be triggered");
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
            ;;(call relay ("op" "noop") [])
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
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();
    drop(connected);
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone()).unwrap();
    drop(connected);

    if let [JValue::String(x)] = client
        .receive_args()
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(x, "called", "spell must be triggered after config update");
    }
}

#[test]
fn spell_set_u32() {
    enable_logs();

    let swarms = make_swarms(1);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = format!(r#"(call "{}" ("return" "") ["called"])"#, client.peer_id);
    let mut config = TriggerConfig::default();
    config.connections.connect = true;

    let spell_id = create_spell(&mut client, &script, config.clone(), hashmap! {});

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    client.send_particle(
        r#"(seq
            (seq
                (call relay (spell_id "get_u32") ["test"] absent)
                (seq
                    (call relay (spell_id "set_u32") ["test" 1])
                    (seq
                        (call relay (spell_id "get_u32") ["test"] one)
                        (seq
                            (call relay (spell_id "set_u32") ["test" 2])
                            (call relay (spell_id "get_u32") ["test"] two)
                        )
                    )
                )
            )
            (call %init_peer_id% ("return" "") [absent one two])
           )"#,
        data,
    );
    let mut result = client.receive_args().wrap_err("receive").unwrap();
    // let mut result = match result {
    //     Some(JValue::Array(array)) => array,
    //     other => panic!("expected array, got {:?}", other),
    // };
    assert_eq!(result.len(), 3);
    let (absent, one, two) = dbg!((result.remove(0), result.remove(0), result.remove(0)));

    assert_eq!(absent["absent"], json!(true));

    assert_eq!(one["absent"], json!(false));
    assert_eq!(one["num"], json!(1));

    assert_eq!(two["absent"], json!(false));
    assert_eq!(two["num"], json!(2));
    // let result = match  {
    //     (absent, one, two) => ,
    //     None => panic!("no results from update_trigger_config particle"),
    //     other => panic!(
    //         "expected JSON String from update_trigger_config particle, got {:?}",
    //         other
    //     ),
    // };
}
