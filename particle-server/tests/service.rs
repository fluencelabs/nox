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

use test_utils::{make_swarms_with_cfg, test_module, ConnectedClient, KAD_TIMEOUT};

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

    let script = format!(
        r#"(seq (
            (call (%current_peer_id% ("add_module" "") (module_bytes module_config) module))
            (seq (
                (call (%current_peer_id% ("add_blueprint" "") (blueprint) blueprint_id))
                (seq (
                    (call (%current_peer_id% ("create" "") (blueprint_id) service_id))
                    (call ("{}" ("" "") ("service_id") client_result))
                ))
            ))
        ))"#,
        client.peer_id
    );

    client.send_particle(
        script,
        json!({
            "module_bytes": base64::encode(test_module()),
            "module_config": config,
            "blueprint": { "name": "blueprint", "dependencies": [module] },
        }),
    );

    let response = client.receive();

    let service_id = response.data.get("service_id").unwrap().as_str().unwrap();
    let script = format!(
        r#"(seq (
            (call (%current_peer_id% ("{}" "greeting") (my_name) greeting))
            (call ("{}" ("" "") (greeting) client_result))
        ))"#,
        service_id, client2.peer_id
    );

    client.send_particle(
        script,
        json!({
            "my_name": "folex"
        }),
    );

    let response = client2.receive();

    assert_eq!(
        response.data.get("greeting").unwrap().as_str().unwrap(),
        "Hi, folex"
    )
}
