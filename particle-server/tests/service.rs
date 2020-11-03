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

use test_utils::{make_swarms_with_cfg, test_module, ConnectedClient, KAD_TIMEOUT};

use fstrings::f;
use maplit::hashmap;
use serde_json::json;
use std::thread::sleep;

#[test]
fn create_service() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let module = "greeting";
    let config = json!(
        {
            "name": module,
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            }
        }
    );

    let script = f!(r#"(seq
            (call relay ("add_module" "") [module_bytes module_config] module)
            (seq
                (call relay ("add_blueprint" "") [blueprint] blueprint_id)
                (seq
                    (call relay ("create" "") [blueprint_id] service_id)
                    (call client ("return" "") [service_id] client_result)
                )
            )
        )"#);
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(test_module())),
        "module_config" => json!(config),
        "blueprint" => json!({ "name": "blueprint", "dependencies": [module] }),
    };

    client.send_particle(script, data);
    let response = client.receive_args();

    let service_id = response[0].as_str().expect("service_id");
    let script = f!(r#"(seq
            (call "{client2.node}" ("{service_id}" "greeting") [my_name] greeting)
            (call "{client2.peer_id}" ("return" "") [greeting] void[])
        )"#);
    client.send_particle(
        script,
        hashmap! {
            "my_name" => json!("folex")
        },
    );

    let response = client2.receive_args();
    assert_eq!(response[0].as_str().unwrap(), "Hi, folex")
}
