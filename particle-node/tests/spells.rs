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
use serde_json::json;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn spell_simple_test() {
    let swarms = make_swarms_with_cfg(1, |mut cfg| {
        cfg.timer_resolution = Duration::from_millis(100);
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
    let data = hashmap! {
        "script" => json!(script.to_string()),
        "period" => json!(1),
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
    };
    client.send_particle(
        r#"
        (seq
            (call relay ("spell" "install") [script period] spell_id)
            (call client ("return" "") [spell_id])
        )"#,
        data.clone(),
    );

    let response = client.receive_args().wrap_err("receive").unwrap();
    let spell_id = response[0].as_str().unwrap().to_string();
    assert_ne!(spell_id.len(), 0);

    sleep(Duration::from_secs(2));
    let mut result = " ".to_string();
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
