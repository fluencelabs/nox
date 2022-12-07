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
use created_swarm::make_swarms_with_cfg;
use eyre::Context;

use maplit::hashmap;
use serde_json::{json, Value as JValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

fn create_spell(
    client: &mut ConnectedClient,
    script: &str,
    period: u32,
    init_data: HashMap<String, String>,
) -> String {
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "period" => json!(period),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "data" => json!(json!(init_data).to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (call relay ("spell" "install") [script data period] spell_id)
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
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.timer_resolution = Duration::from_millis(20);
        cfg
    });
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

    let spell_id = create_spell(&mut client, script, 0, hashmap! {});

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
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.timer_resolution = Duration::from_millis(20);
        cfg
    });
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let failing_script = r#"
        (xor
            (call %init_peer_id% ("srv" "remove") ["non_existent_srv_id"])
            (call %init_peer_id% ("errorHandlingSrv" "error") [%last_error% 1])        
        )"#;
    let spell_id = create_spell(&mut client, failing_script, 0, hashmap! {});

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
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.timer_resolution = Duration::from_millis(100);
        cfg
    });
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

    let spell_id = create_spell(
        &mut client,
        script,
        0,
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
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.timer_resolution = Duration::from_millis(100);
        cfg
    });
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

    let spell_id = create_spell(&mut client, script, 0, hashmap! {});

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
