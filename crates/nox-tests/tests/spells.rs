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
use fluence_keypair::KeyPair;
use maplit::hashmap;
use serde_json::{json, Value as JValue};

use connected_client::ConnectedClient;
use created_swarm::system_services_config::{DeciderConfig, SystemServicesConfig};
use created_swarm::{make_swarms, make_swarms_with_cfg};
use fluence_spell_dtos::trigger_config::{ClockConfig, TriggerConfig};
use fs_utils::make_tmp_dir_peer_id;
use service_modules::load_module;
use spell_event_bus::api::{TriggerInfo, TriggerInfoAqua, MAX_PERIOD_SEC};
use test_utils::{create_service, create_service_worker};

type SpellId = String;
type WorkerPeerId = String;

async fn create_worker(client: &mut ConnectedClient, deal_id: Option<String>) -> WorkerPeerId {
    let data = hashmap! {
        "deal_id" => deal_id.map(JValue::String).unwrap_or(JValue::Null),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
            (seq
                (xor
                    (call relay ("worker" "create") [deal_id] worker_peer_id)
                    (call relay ("worker" "get_peer_id") [deal_id] worker_peer_id)
                )
                (call client ("return" "") [worker_peer_id])
            )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let worker_id = response[0].as_str().unwrap().to_string();
    assert_ne!(worker_id.len(), 0);

    worker_id
}

async fn create_spell_with_alias(
    client: &mut ConnectedClient,
    script: &str,
    config: TriggerConfig,
    init_data: JValue,
    deal_id: Option<String>,
    alias: String,
) -> (SpellId, WorkerPeerId) {
    let worker_id = create_worker(client, deal_id).await;
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "worker_id" => json!(worker_id.clone()),
        "alias" => json!(alias),
        "data" => init_data,
    };

    let response = client
        .execute_particle(
            r#"
        (seq
            (call relay ("op" "noop") [])            
            (seq
                (call worker_id ("spell" "install") [script data config alias] spell_id)
                (call client ("return" "") [spell_id])
            )
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);

    (spell_id, worker_id)
}

async fn create_spell(
    client: &mut ConnectedClient,
    script: &str,
    config: TriggerConfig,
    init_data: JValue,
    deal_id: Option<String>,
) -> (SpellId, WorkerPeerId) {
    let worker_id = create_worker(client, deal_id).await;
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "worker_id" => json!(worker_id.clone()),
        "data" => init_data,
    };

    let response = client
        .execute_particle(
            r#"
        (seq
            (call relay ("op" "noop") [])            
            (seq
                (call worker_id ("spell" "install") [script data config] spell_id)
                (call client ("return" "") [spell_id])
            )
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);

    (spell_id, worker_id)
}

fn make_clock_config(period_sec: u32, start_sec: u32, end_sec: u32) -> TriggerConfig {
    TriggerConfig {
        clock: ClockConfig {
            start_sec,
            end_sec,
            period_sec,
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn spell_simple_test() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let config = make_clock_config(0, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();
    let result = response[0].as_str().unwrap().to_string();
    assert!(response[1]["success"].as_bool().unwrap());
    let counter = response[1]["num"].as_u64().unwrap();

    assert_eq!(result, script);
    assert_ne!(counter, 0);
}

#[tokio::test]
async fn spell_error_handling_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let failing_script = r#"
        (xor
            (call %init_peer_id% ("srv" "remove") ["non_existent_srv_id"])
            (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])        
        )"#;

    let config = make_clock_config(2, 3, 0);
    let (spell_id, worker_id) =
        create_spell(&mut client, failing_script, config, json!({}), None).await;

    // let's retrieve error from the first spell particle
    let particle_id = format!("spell_{}_{}", spell_id, 0);
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "particle_id" => json!(particle_id),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
    };

    tokio::time::sleep(Duration::from_secs(3)).await;

    let response = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_errors") [particle_id] result)
            )
            (call client ("return" "") [result])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let mut result = vec![];
    if !response[0].as_array().unwrap().is_empty() {
        result = response[0].as_array().unwrap().clone();
    }

    assert_eq!(result.len(), 1);

    swarms
        .into_iter()
        .map(|s| s.exit_outlet.send(()))
        .for_each(drop);
}

#[tokio::test]
async fn spell_args_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let config = make_clock_config(1, 1, 0);

    let expected_value = json!({"a": "b", "c": 1});
    create_spell(
        &mut client,
        &script,
        config,
        json!({ "key": expected_value }),
        None,
    )
    .await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();
    assert_eq!(response[0], expected_value);
}

#[tokio::test]
async fn spell_return_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let config = make_clock_config(1, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();
    let value = response[0].as_str().unwrap().to_string();

    assert_eq!(value, "value");
}

#[tokio::test]
async fn spell_install_alias() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let alias = "spell_alias".to_string();
    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] actual_spell_id)
                (call %init_peer_id% ("srv" "resolve_alias") ["{}"] aliased_spell_id)
            )
            (call "{}" ("return" "") [actual_spell_id aliased_spell_id])
        )"#,
        alias, client.peer_id
    );

    let config = make_clock_config(1, 1, 0);
    create_spell_with_alias(&mut client, &script, config, json!({}), None, alias).await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();
    let actual_spell_id = response[0].as_str().unwrap().to_string();
    let aliased_spell_id = response[1].as_str().unwrap().to_string();

    assert_eq!(actual_spell_id, aliased_spell_id);
}

// Check that oneshot spells are actually executed and executed only once
#[tokio::test]
async fn spell_run_oneshot() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"
        (seq
            (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
            (call %init_peer_id% (spell_id "set_string") ["result" "done"])
        )"#;

    // Note that when period is 0, the spell is executed only once
    let config = make_clock_config(0, 1, 0);
    let (spell_id, worker_id) = create_spell(&mut client, script, config, json!({}), None).await;

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_u32") ["counter"] counter)
            )
            (call client ("return" "") [counter])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 1);
    }
}

// The config considered empty if start_sec is 0. In this case we don't schedule a spell.
// Script installation will fail because no triggers configured.
#[tokio::test]
async fn spell_install_ok_empty_config() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;

    // Note that when period is 0, the spell is executed only once
    let config = TriggerConfig::default();
    let (spell_id, worker_id) = create_spell(&mut client, script, config, json!({}), None).await;

    // The spell should be installed, but should not be subscribed to any triggers
    // We cannot truly check that the spell isn't subscribed to anything right now, but we can check that
    // it's counter is zero on different occasions:

    // 1. Check that the spell wasn't executed immediately after installation (the case of `start_sec` <= now)
    let response = client
        .execute_particle(
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
        )
        .await
        .unwrap();

    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 0);
    }
    // 2. Connect and disconnect a client to the same node. The spell should not be executed
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    drop(connected);

    let response = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    if response[0]["success"].as_bool().unwrap() {
        let counter = response[0]["num"].as_u64().unwrap();
        assert_eq!(counter, 0);
    }

    // 3. We cannot check that it's not scheduled to run in the future, but it's ok for now.
}

#[tokio::test]
async fn spell_install_fail_large_period() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();
    let worker_id = create_worker(&mut client, None).await;

    // Note that when period is 0, the spell is executed only once
    let config = make_clock_config(MAX_PERIOD_SEC + 1, 1, 0);

    let data = hashmap! {
        "worker_id" => json!(worker_id),
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };
    let result = client
        .execute_particle(
            r#"
        (xor
            (seq
                (call relay ("op" "noop") [])            
                (call worker_id ("spell" "install") [script data config] spell_id)
            )
            (call client ("return" "") [%last_error%.$.message])
        )"#,
            data,
        )
        .await
        .unwrap();

    if let [JValue::String(error_msg)] = result.as_slice() {
        let msg = "Local service error, ret_code is 1, error message is '\"Error: invalid config: period is too big.";
        assert!(error_msg.starts_with(msg));
    }
}

// Also the config considered invalid if the end_sec is in the past.
// In this case we don't schedule a spell and return error.
#[tokio::test]
async fn spell_install_fail_end_sec_past() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    // Note that when period is 0, the spell is executed only once
    let config = make_clock_config(0, 10, 1);
    let worker_id = create_worker(&mut client, None).await;

    let data = hashmap! {
        "worker_id" => json!(worker_id),
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };

    let result = client
        .execute_particle(
            r#"
        (xor
            (seq
                (call relay ("op" "noop") [])            
                (call worker_id ("spell" "install") [script data config] spell_id)
            )
            (call client ("return" "") [%last_error%.$.message])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();
    if let [JValue::String(error_msg)] = result.as_slice() {
        let expected = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past";
        assert!(
            error_msg.starts_with(expected),
            "expected:\n{expected}\ngot:\n{error_msg}"
        );
    }
}

// Also the config considered invalid if the end_sec is less than start_sec.
// In this case we don't schedule a spell and return error.
#[tokio::test]
async fn spell_install_fail_end_sec_before_start() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let empty: HashMap<String, String> = HashMap::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Note that when period is 0, the spell is executed only once
    let config = make_clock_config(0, now as u32 + 100, now as u32 + 90);
    let worker_id = create_worker(&mut client, None).await;

    let data = hashmap! {
        "worker_id" => json!(worker_id),
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(empty).to_string()),
    };
    let result = client
        .execute_particle(
            r#"
        (xor
            (seq
                (call relay ("op" "noop") [])            
                (call worker_id ("spell" "install") [script data config] spell_id)
            )
            (call client ("return" "") [%last_error%.$.message])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::String(error_msg)] = result.as_slice() {
        let expected = "Local service error, ret_code is 1, error message is '\"Error: invalid config: end_sec is less than start_sec or in the past";
        assert!(
            error_msg.starts_with(expected),
            "expected:\n{expected}\ngot:\n{error_msg}"
        );
    }
}

#[tokio::test]
async fn spell_store_trigger_config() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let config = make_clock_config(13, 10, 0);

    let (spell_id, worker_id) =
        create_spell(&mut client, script, config.clone(), json!({}), None).await;
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker (spell_id "get_trigger_config") [] config)
            )
            (call client ("return" "") [config])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    if response[0]["success"].as_bool().unwrap() {
        let result_config = serde_json::from_value(response[0]["config"].clone()).unwrap();
        assert_eq!(config, result_config);
    }
}

#[tokio::test]
async fn spell_remove() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;
    let config = make_clock_config(2, 1, 0);
    let (spell_id, worker_id) = create_spell(&mut client, script, config, json!({}), None).await;

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "worker" => json!(worker_id.clone())
    };

    let result = client
        .execute_particle(
            r#"
    (seq
        (call relay ("spell" "list") [] list)
        (call client ("return" "") [list])
    )"#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::String(result_spell_id)] = result.as_slice() {
        assert_eq!(&spell_id, result_spell_id);
    }

    let result = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker ("spell" "remove") [spell_id])
            )
            (seq
                (call relay ("spell" "list") [] list)
                (call client ("return" "") [list])
            )
        )
        "#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::Array(created_spells)] = result.as_slice() {
        assert!(created_spells.is_empty(), "no spells should exist");
    }
}

#[tokio::test]
async fn spell_remove_by_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let config = make_clock_config(2, 1, 0);
    let (spell_id, _) = create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::Array(before), JValue::Array(after)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(before.len(), 1);
        assert_eq!(before[0], spell_id);
        assert!(after.is_empty());
    }
}

#[tokio::test]
async fn spell_remove_spell_as_service() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;

    let config = make_clock_config(2, 1, 0);
    let (spell_id, _) = create_spell(&mut client, script, config, json!({}), None).await;

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };

    let result = client
        .execute_particle(
            r#"
        (xor
            (call relay ("srv" "remove") [spell_id])
            (call client ("return" "") [%last_error%.$.message])
        )
        "#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::String(msg)] = result.as_slice() {
        let expected = "cannot call function 'remove_service': cannot remove a spell";
        assert!(
            msg.contains(expected),
            "should contain `{expected}`, given msg `{msg}`"
        );
    }
}

#[tokio::test]
async fn spell_remove_service_as_spell() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let service = create_service(
        &mut client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share").expect("load module"),
    )
    .await;

    let data = hashmap! {
        "service_id" => json!(service.id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };

    let result = client
        .execute_particle(
            r#"
        (xor
            (call relay ("spell" "remove") [service_id])
            (call client ("return" "") [%last_error%.$.message])
        )
        "#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::String(msg)] = result.as_slice() {
        let expected = "cannot call function 'remove_spell': the service isn't a spell";
        assert!(
            msg.contains(expected),
            "should contain `{expected}`, given msg `{msg}`"
        );
    }
}

#[tokio::test]
async fn spell_call_by_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

                (call "{}" ("return" "") [counter.$.num])
            )
        )"#,
        client.peer_id
    );

    let config = make_clock_config(2, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::Number(counter)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_ne!(counter.as_i64().unwrap(), 0);
    }
}

#[tokio::test]
async fn spell_trigger_connection_pool() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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
    let (spell_id1, _) = create_spell(
        &mut client,
        &script,
        config,
        json!({}),
        Some("deal_id1".to_string()),
    )
    .await;

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    let (spell_id2, _) = create_spell(
        &mut client,
        &script,
        config,
        json!({}),
        Some("deal_id2".to_string()),
    )
    .await;

    // This connect should trigger the spell
    let connect_num = 5;
    for _ in 0..connect_num {
        ConnectedClient::connect_to(swarms[0].multiaddr.clone())
            .await
            .unwrap();
    }

    let mut spell1_counter = 0;
    let mut spell2_counter = 0;

    // we must receive `connect_num` messages from spell1 subscribed on connect and `connect_num` messages
    // from spell1 subscribed on disconnect, so 2 * `connect_num` messages in total
    for _ in 0..2 * connect_num {
        if let [spell_reply] = client
            .receive_args()
            .await
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

#[tokio::test]
async fn spell_timer_trigger_mailbox_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();
    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("getDataSrv" "trigger") [] trigger)
            )
            (call "{}" ("return" "") [trigger])
        )
    "#,
        client.peer_id
    );

    let config = make_clock_config(0, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    let value = client.receive_args().await.wrap_err("receive").unwrap()[0]
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

#[tokio::test]
async fn spell_connection_pool_trigger_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (xor
            (seq
                (seq
                    (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                    (seq
                        (call %init_peer_id% ("getDataSrv" "trigger") [] trigger)
                        (call %init_peer_id% ("run-console" "print") ["getDataSrv, trigger:" trigger])
                    )
                )
                (call "{}" ("return" "") [trigger])
            )
            (call %init_peer_id% ("run-console" "print") ["herror" %last_error%])
        )
    "#,
        client.peer_id
    );

    let mut config = TriggerConfig::default();
    config.connections.disconnect = true;
    create_spell(&mut client, &script, config.clone(), json!({}), None).await;
    log::info!("created spell");

    let disconnected_client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();
    let disconnected_client_peer_id = disconnected_client.peer_id;
    log::info!("will disconnect");
    disconnected_client.client.stop();

    log::info!("Main client: {}", client.peer_id);
    log::info!("Disconnected client: {}", disconnected_client_peer_id);

    loop {
        client.timeout = Duration::from_secs(30);
        let value = client.receive_args().await.wrap_err("receive").unwrap()[0]
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

        if peer["peer_id"] == JValue::String(client.peer_id.to_string()) {
            // we got a Peer event about `client`, just ignore it and wait for the next trigger
            continue;
        }

        assert_eq!(
            peer["peer_id"].as_str().unwrap(),
            disconnected_client_peer_id.to_base58()
        );
        assert_eq!(peer["connected"].as_bool().unwrap(), false);

        break;
    }
}

#[tokio::test]
async fn spell_set_u32() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(r#"(call "{}" ("return" "") ["called"])"#, client.peer_id);
    let mut config = TriggerConfig::default();
    config.connections.connect = true;

    let (spell_id, worker_id) =
        create_spell(&mut client, &script, config.clone(), json!({}), None).await;

    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "worker" => json!(worker_id),
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    let mut result = client
        .execute_particle(
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
        )
        .await
        .unwrap();
    assert_eq!(result.len(), 3);
    let (absent, one, two) = (result.remove(0), result.remove(0), result.remove(0));

    assert_eq!(absent["absent"], json!(true));

    assert_eq!(one["absent"], json!(false));
    assert_eq!(one["num"], json!(1));

    assert_eq!(two["absent"], json!(false));
    assert_eq!(two["num"], json!(2));
}

#[tokio::test]
async fn spell_peer_id_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
            (call "{}" ("return" "") [%init_peer_id%])
        "#,
        client.peer_id
    );

    let config = make_clock_config(0, 1, 0);
    let (_, worker_peer_id) = create_spell(&mut client, &script, config, json!({}), None).await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();

    let result = response[0].as_str().unwrap().to_string();

    assert_eq!(result, worker_peer_id);
}

#[tokio::test]
async fn spell_update_config() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"(seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("getDataSrv" "trigger") [] trigger)
             )
            (call "{}" ("return" "") [trigger])
        )"#,
        client.peer_id
    );
    let mut config = TriggerConfig::default();
    config.connections.connect = true;
    let (spell_id, worker_id) = create_spell(&mut client, &script, config, json!({}), None).await;
    let connected = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .unwrap();

    if let [trigger] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let info: TriggerInfoAqua = serde_json::from_str(&trigger.to_string()).unwrap();
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
        "worker" => json!(worker_id),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    let mut result = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker ("spell" "update_trigger_config") [spell_id config])
            )
            (call %init_peer_id% ("return" "") ["done"])
           )"#,
            data,
        )
        .await
        .unwrap();
    let result = result.pop();
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

    if let [trigger] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let info: TriggerInfoAqua = serde_json::from_str(&trigger.to_string()).unwrap();
        let info: TriggerInfo = info.into();
        assert_matches!(info, TriggerInfo::Peer(p) if !p.connected, "spell must be triggered by the `disconnected` event");
    } else {
        panic!("wrong result from spell, expect trigger info with the `disconnected` event");
    }
}

#[tokio::test]
async fn spell_update_config_stopped_spell() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"(seq
            (seq
                (call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)
                (call %init_peer_id% ("getDataSrv" "trigger") [] trigger)
             )
            (call "{}" ("return" "") [trigger])
        )"#,
        client.peer_id
    );
    // create periodic spell
    let config = TriggerConfig::default();
    let (spell_id, worker_id) = create_spell(&mut client, &script, config, json!({}), None).await;

    // Update trigger config to do something.
    let config = make_clock_config(0, 1, 0);
    let data = hashmap! {
        "spell_id" => json!(spell_id),
        "relay" => json!(client.node.to_string()),
        "worker" => json!(worker_id),
        "client" => json!(client.peer_id.to_string()),
        "config" => json!(config),
    };
    let mut result = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("op" "noop") [])
                (call worker ("spell" "update_trigger_config") [spell_id config])
            )
            (call %init_peer_id% ("return" "") ["done"])
        )"#,
            data,
        )
        .await
        .unwrap();
    let result = result.pop();
    let result = match result {
        Some(JValue::String(result)) => result,
        None => panic!("no results from update_trigger_config particle"),
        other => panic!(
            "expected JSON String from update_trigger_config particle, got {:?}",
            other
        ),
    };
    assert_eq!(result, "done", "spell must be updated");

    if let [trigger] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        let info: TriggerInfoAqua = serde_json::from_str(&trigger.to_string()).unwrap();
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

#[tokio::test]
async fn resolve_alias_wrong_worker() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
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

    let config = make_clock_config(2, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::String(error)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert!(error.starts_with("Local service error, ret_code is 1, error message is '\"Error: Service with alias 'alias' is not found on worker"));
    }
}

#[tokio::test]
async fn resolve_global_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let tetraplets_service = create_service(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
    )
    .await;

    client
        .send_particle(
            r#"(call relay ("srv" "add_alias") ["alias" service])"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "service" => json!(tetraplets_service.id),
            },
        )
        .await;

    let script = format!(
        r#"
        (seq
            (call %init_peer_id% ("srv" "resolve_alias") ["alias"] resolved)
            (call "{0}" ("return" "") [resolved])
        )"#,
        client.peer_id
    );

    let config = make_clock_config(2, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::String(resolved)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*resolved, tetraplets_service.id);
    }
}

#[tokio::test]
async fn worker_sig_test() {
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.enabled_system_services = vec!["registry".to_string()];
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (seq
                    (call %init_peer_id% ("registry" "get_record_metadata_bytes") ["key_id" "" 0 "" "" [] [] []] data)
                    (call %init_peer_id% ("sig" "get_peer_id") [] peer_id)
                )
                (seq
                    (call %init_peer_id% ("sig" "sign") [data] sig_result)
                    (call %init_peer_id% ("sig" "verify") [sig_result.$.signature.[0]! data] result)
                )
            )
            (call "{0}" ("op" "return") [sig_result result peer_id])
        )
       "#,
        client.peer_id
    );

    let config = make_clock_config(2, 1, 0);
    let (_, worker_id) = create_spell(&mut client, &script, config, json!({}), None).await;

    use serde_json::Value::Bool;
    use serde_json::Value::Object;
    use serde_json::Value::String;

    if let [Object(sig_result), Bool(result), String(peer_id)] =
        client.receive_args().await.unwrap().as_slice()
    {
        assert!(sig_result["success"].as_bool().unwrap());
        assert!(result);
        assert_eq!(*peer_id, worker_id);
    } else {
        panic!("incorrect args: expected two arguments")
    }
}

#[tokio::test]
async fn spell_relay_id_test() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (call %init_peer_id% ("getDataSrv" "-relay-") [] -relay-)
                (call -relay- ("op" "identity") [-relay-] also_relay)
            )
            (call "{}" ("return" "") [also_relay])
        )"#,
        client.peer_id
    );

    let config = make_clock_config(1, 1, 0);
    create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::String(relay_id)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*relay_id, client.node.to_base58());
    } else {
        panic!("expected one string result")
    }
}

#[tokio::test]
async fn spell_create_worker_twice() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
    };
    client
        .send_particle(
            r#"
        (xor
            (seq
                (seq
                    (call relay ("worker" "create") ["deal_id"] worker_peer_id)
                    (call relay ("worker" "get_peer_id") ["deal_id"] get_worker_peer_id)
                )
                (seq
                    (call relay ("worker" "create") ["deal_id"] failed_create)
                    (call client ("return" "") ["test failed"])
                )
            )
            (call client ("return" "") [%last_error%.$.message worker_peer_id get_worker_peer_id])
        )"#,
            data.clone(),
        )
        .await;

    let response = client.receive_args().await.wrap_err("receive").unwrap();
    let error_msg = response[0].as_str().unwrap().to_string();
    assert!(error_msg.contains("Worker for deal_id already exists"));
    let worker_id = response[1].as_str().unwrap().to_string();
    assert_ne!(worker_id.len(), 0);
    let get_worker_id = response[2].as_str().unwrap().to_string();
    assert_eq!(worker_id, get_worker_id);
}

#[tokio::test]
async fn spell_install_root_scope() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let script = r#"(call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)"#;

    let config = make_clock_config(0, 1, 0);

    let data = hashmap! {
        "script" => json!(script.to_string()),
        "config" => json!(config),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!({})
    };
    let response = client
        .execute_particle(
            r#"
        (seq
            (seq
                (call relay ("spell" "install") [script data config] spell_id)
                (call relay ("srv" "info") [spell_id] info)
            )
            (call client ("return" "") [spell_id info.$.worker_id])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();
    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);
    let worker_id = response[1].as_str().unwrap().to_string();
    assert_eq!(worker_id, client.node.to_base58());
}

#[tokio::test]
async fn spell_create_worker_same_deal_id_different_peer() {
    let swarms = make_swarms(1).await;
    let mut client1 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let data = hashmap! {
        "client" => json!(client1.peer_id.to_string()),
        "relay" => json!(client1.node.to_string()),
    };
    let response = client1
        .execute_particle(
            r#"
        (seq
            (call relay ("worker" "create") ["deal_id"] worker_peer_id)
            (call client ("return" "") [worker_peer_id])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let worker_id = response[0].as_str().unwrap().to_string();
    assert_ne!(worker_id.len(), 0);

    let data = hashmap! {
        "client" => json!(client2.peer_id.to_string()),
        "relay" => json!(client2.node.to_string()),
    };
    let response = client2
        .execute_particle(
            r#"
        (xor
            (seq
                (call relay ("worker" "create") ["deal_id"] worker_peer_id)
                (call client ("return" "") ["test_failed"])
            )
            (call client ("return" "") [%last_error%.$.message])
        )"#,
            data.clone(),
        )
        .await
        .unwrap();

    let error_msg = response[0].as_str().unwrap().to_string();
    assert!(error_msg.contains("Worker for deal_id already exists"));
}

#[tokio::test]
async fn create_remove_worker() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("getDataSrv" "spell_id") [] spell_id)"#;
    let config = make_clock_config(0, 1, 0);

    let (spell_id, worker_id) = create_spell(&mut client, &script, config, json!({}), None).await;
    let service = create_service_worker(
        &mut client,
        "file_share",
        load_module("tests/file_share/artifacts", "file_share").expect("load module"),
        worker_id.clone(),
    )
    .await;
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "worker_id" => json!(worker_id.clone()),
        "spell_id" => json!(spell_id.clone()),
        "srv_id" => json!(service.id.clone()),
    };
    client
        .send_particle(
            r#"
        (xor
            (seq
                (seq
                    (call relay ("op" "noop") [])
                    (call worker_id ("srv" "list") [] before)
                )
                (seq
                    (seq
                        (call relay ("worker" "remove") [worker_id])
                        (xor
                            (call relay ("srv" "info") [spell_id] info1)
                            (call relay ("op" "identity") [%last_error%.$.message] err1)
                        )
                    )
                    (seq
                        (xor
                            (call relay ("srv" "info") [srv_id] info2)
                            (call relay ("op" "identity") [%last_error%.$.message] err2)
                        )
                        (call client ("return" "") [before err1 err2])
                    )
                )
            )
            (call client ("return" "") [%last_error%.$.message])
        )
    "#,
            data.clone(),
        )
        .await;

    if let [JValue::Array(before), JValue::String(spell_err), JValue::String(srv_err)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(before.len(), 2);

        let before: Vec<String> = before
            .into_iter()
            .map(|s| s.get("id").unwrap().as_str().unwrap().to_string())
            .collect();
        assert!(before.contains(&spell_id));
        assert!(before.contains(&service.id));
        assert!(spell_err.contains(&format!("Service with id '{spell_id}' not found")));
        assert!(srv_err.contains(&format!("Service with id '{}' not found", service.id)));
    } else {
        panic!("expected array and two strings")
    }
}

#[tokio::test]
async fn spell_update_trigger_by_alias() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
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
    let (spell_id, worker) = create_spell(&mut client, &script, config, json!({}), None).await;

    let mut new_config = TriggerConfig::default();
    new_config.connections.connect = true;
    new_config.connections.disconnect = true;

    let id = client
        .send_particle(
            r#"(seq
                    (seq
                        (call relay ("op" "noop") [])
                        (call worker ("srv" "add_alias") ["alias" spell_id])
                    )
                    (seq
                        (call worker ("spell" "update_trigger_config") ["alias" config])
                        (call %init_peer_id% ("return" "") ["ok"])
                    )
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "worker" => json!(worker),
                "spell_id" => json!(spell_id.clone()),
                "config" => json!(new_config)
            },
        )
        .await;

    client.wait_particle_args(id).await.unwrap();

    // This connect should trigger the spell
    let connect_num = 3;
    for _ in 0..connect_num {
        ConnectedClient::connect_to(swarms[0].multiaddr.clone())
            .await
            .unwrap();
    }

    let mut trigger_counter = 0;

    // we must receive `connect_num` messages on connect and `connect_num` messages
    // on disconnect, so 2 * `connect_num` messages in total
    for _ in 0..2 * connect_num {
        if let [spell_reply] = client
            .receive_args()
            .await
            .wrap_err("receive")
            .unwrap()
            .as_slice()
        {
            let given_spell_id = spell_reply.as_str().unwrap();
            assert_eq!(spell_id, given_spell_id);

            trigger_counter += 1;
        }
    }

    assert_eq!(
        trigger_counter,
        connect_num * 2,
        "spell must be triggered {connect_num} * 2 times"
    );
}

#[tokio::test]
async fn test_worker_list() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let worker_id1 = create_worker(&mut client, Some("deal_id1".to_string())).await;
    let worker_id2 = create_worker(&mut client, None).await;

    client
        .send_particle(
            r#"(seq
                    (call relay ("worker" "list") [] result)
                    (call client ("return" "") [result])
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string())
            },
        )
        .await;

    if let [JValue::Array(workers)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(workers.len(), 2);

        let workers: Vec<String> = workers
            .into_iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(workers.contains(&worker_id1));
        assert!(workers.contains(&worker_id2));
    } else {
        panic!("expected one array result")
    }
}

#[tokio::test]
async fn test_spell_list() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = r#"(call %init_peer_id% ("peer" "identify") [] x)"#;

    let config = make_clock_config(0, 1, 0);
    let (spell_id1, _worker_id1) = create_spell(
        &mut client,
        script,
        config.clone(),
        json!({}),
        Some("deal_id_1".to_string()),
    )
    .await;

    let (spell_id2, worker_id1) = create_spell(
        &mut client,
        script,
        config.clone(),
        json!({}),
        Some("deal_id_1".to_string()),
    )
    .await;

    let (spell_id3, worker_id2) = create_spell(
        &mut client,
        script,
        config.clone(),
        json!({}),
        Some("deal_id_2".to_string()),
    )
    .await;

    client
        .send_particle(
            r#"(seq
                    (seq
                        (call relay ("op" "noop") [])
                        (seq
                            (call worker1 ("spell" "list") [] worker1_spells)
                            (call worker2 ("spell" "list") [] worker2_spells)
                        )
                    )
                    (call client ("return" "") [worker1_spells worker2_spells])
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
                "worker1" => json!(worker_id1),
                "worker2" => json!(worker_id2),
            },
        )
        .await;

    if let [JValue::Array(worker1_spells), JValue::Array(worker2_spells)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(worker1_spells.len(), 2);
        assert_eq!(worker2_spells.len(), 1);
        let worker1_spells: Vec<String> = worker1_spells
            .into_iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert!(worker1_spells.contains(&spell_id1));
        assert!(worker1_spells.contains(&spell_id2));
        assert!(worker2_spells[0].as_str().unwrap().eq(&spell_id3));
    } else {
        panic!("expected one array result")
    }
}

#[tokio::test]
async fn spell_call_by_default_alias() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let script = format!(
        r#"
        (seq
            (seq
                (seq
                    (call %init_peer_id% ("spell" "get_u32") ["counter"] counter1)
                    (seq
                        (call %init_peer_id% ("srv" "resolve_alias") ["spell"] spell_id1)
                        (xor
                            (call %init_peer_id% ("srv" "add_alias") ["spell" spell_id1])
                            (call %init_peer_id% ("op" "identity") [%last_error%.$.message] error1)
                        )
                    )
                )
                (seq
                    (call %init_peer_id% ("self" "get_u32") ["counter"] counter2)
                    (seq
                        (call %init_peer_id% ("srv" "resolve_alias") ["self"] spell_id2)
                        (xor
                            (call %init_peer_id% ("srv" "add_alias") ["self" spell_id2])
                            (call %init_peer_id% ("op" "identity") [%last_error%.$.message] error2)
                        )
                    )
                )
            )
            (call "{}" ("return" "") [counter1.$.num spell_id1 error1 counter2.$.num spell_id2 error2])
        )"#,
        client.peer_id
    );

    let config = make_clock_config(2, 1, 0);
    let (expected_spell_id, _) = create_spell(&mut client, &script, config, json!({}), None).await;

    if let [JValue::Number(counter1), JValue::String(spell_id1), JValue::String(error1), JValue::Number(counter2), JValue::String(spell_id2), JValue::String(error2)] =
        client
            .receive_args()
            .await
            .wrap_err("receive")
            .unwrap()
            .as_slice()
    {
        assert_ne!(counter1.as_i64().unwrap(), 0);
        assert_eq!(spell_id1, &expected_spell_id);
        assert!(error1.contains("Cannot add alias 'spell' because it is reserved"));

        assert_ne!(counter2.as_i64().unwrap(), 0);
        assert_eq!(spell_id2, &expected_spell_id);
        assert!(error2.contains("Cannot add alias 'self' because it is reserved"));
    } else {
        panic!("expected (int, str, str, int, str, str) result");
    }
}

#[tokio::test]
async fn get_worker_peer_id_opt() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let data = hashmap! {
        "relay" => json!(client.node.to_string()),
        "client" => json!(client.peer_id.to_string()),
    };
    let response = client
        .execute_particle(
            r#"
            (seq
                (seq
                    (call relay ("worker" "get_worker_id") ["deal_id"] worker_peer_id_before)
                    (call relay ("worker" "create") ["deal_id"] worker_peer_id)
                )
                (seq
                    (call relay ("worker" "get_worker_id") ["deal_id"] worker_peer_id_after)
                    (call client ("return" "") [worker_peer_id_before worker_peer_id worker_peer_id_after])
                )
            )"#,
            data.clone(),
        )
        .await
        .unwrap();

    if let [JValue::Array(worker_id_before), JValue::String(worker_peer_id), JValue::Array(worker_id_after)] =
        response.as_slice()
    {
        assert_eq!(worker_id_before.len(), 0);
        assert_eq!(worker_id_after.len(), 1);
        let worker_id = worker_id_after[0].as_str().unwrap().to_string();
        assert_eq!(worker_id, *worker_peer_id);
    } else {
        panic!("expected result")
    }
}

#[tokio::test]
async fn set_alias_by_worker_creator() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let worker_id = create_worker(&mut client, None).await;

    let tetraplets_service = create_service_worker(
        &mut client,
        "tetraplets",
        load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module"),
        worker_id.clone(),
    )
    .await;

    client
        .send_particle(
            r#"(seq
                    (seq
                        (call relay ("op" "noop") []) 
                        (call worker ("srv" "add_alias") ["alias" service])
                    )
                    (seq
                        (call worker ("srv" "resolve_alias_opt") ["alias"] resolved)
                        (call client ("return" "") [resolved.$.[0]!])
                    )
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
                "service" => json!(tetraplets_service.id),
                "worker" => json!(worker_id),
            },
        )
        .await;

    if let [JValue::String(resolved)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*resolved, tetraplets_service.id);
    }
}

#[tokio::test]
async fn test_decider_api_endpoint_rewrite() {
    let expected_endpoint = "test1".to_string();
    let swarm_keypair = KeyPair::generate_ed25519();
    let swarm_dir = make_tmp_dir_peer_id(swarm_keypair.get_peer_id().to_string());
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.keypair = swarm_keypair.clone();
        cfg.tmp_dir = Some(swarm_dir.clone());
        cfg.enabled_system_services = vec!["decider".to_string()];
        cfg.override_system_services_config = Some(SystemServicesConfig {
            enable: vec![],
            aqua_ipfs: Default::default(),
            decider: DeciderConfig {
                network_api_endpoint: expected_endpoint.clone(),
                ..Default::default()
            },
            registry: Default::default(),
            connector: Default::default(),
        });
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"(seq
                    (call relay ("decider" "get_string") ["chain"] chain_info_str)
                    (seq
                        (call relay ("json" "parse") [chain_info_str.$.str] chain_info)
                        (call client ("return" "") [chain_info.$.api_endpoint])
                    )
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        )
        .await;

    if let [JValue::String(endpoint)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*endpoint, expected_endpoint);
    }

    // stop swarm
    swarms
        .into_iter()
        .map(|s| s.exit_outlet.send(()))
        .for_each(drop);

    let another_endpoint = "another_endpoint_test".to_string();
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.keypair = swarm_keypair.clone();
        cfg.tmp_dir = Some(swarm_dir.clone());
        cfg.enabled_system_services = vec!["decider".to_string()];
        cfg.override_system_services_config = Some(SystemServicesConfig {
            enable: vec![],
            aqua_ipfs: Default::default(),
            decider: DeciderConfig {
                network_api_endpoint: another_endpoint.clone(),
                ..Default::default()
            },
            registry: Default::default(),
            connector: Default::default(),
        });
        cfg
    })
    .await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    client
        .send_particle(
            r#"(seq
                    (call relay ("decider" "get_string") ["chain"] chain_info_str)
                    (seq
                        (call relay ("json" "parse") [chain_info_str.$.str] chain_info)
                        (call client ("return" "") [chain_info.$.api_endpoint])
                    )
                )"#,
            hashmap! {
                "relay" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string()),
            },
        )
        .await;

    if let [JValue::String(endpoint)] = client
        .receive_args()
        .await
        .wrap_err("receive")
        .unwrap()
        .as_slice()
    {
        assert_eq!(*endpoint, another_endpoint);
    }
}

#[tokio::test]
async fn test_activate_deactivate() {
    let swarms = make_swarms(1).await;
    let mut client = ConnectedClient::connect_with_keypair(
        swarms[0].multiaddr.clone(),
        Some(swarms[0].management_keypair.clone()),
    )
    .await
    .wrap_err("connect client")
    .unwrap();

    let deal_id = "deal-id-1".to_string();

    let config = make_clock_config(2, 1, 0);
    let (spell_id, worker_id) =
        create_spell(&mut client, "()", config, json!({}), Some(deal_id.clone())).await;
}
