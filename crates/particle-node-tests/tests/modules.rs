/*
 * Copyright 2023 Fluence Labs Limited
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
use service_modules::load_module;
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use maplit::hashmap;
use serde_json::json;

#[tokio::test]
async fn test_add_module_mounted_binaries() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .expect("connect client");
    let module = load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module");

    let config = json!(
        {
            "name": "tetraplets",
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            },
            "mounted_binaries": json!({"cmd": "/usr/bin/curl"})
        });

   let script = r#"
    (xor
       (seq
           (call node ("dist" "add_module") [module_bytes module_config])
           (call client ("return" "") ["ok"])
       )
       (call client ("return" "") [%last_error%.$.message])
    )
   "#;

    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64.encode(&module)),
        "module_config" => config,
    };

    let response = client.execute_particle(script, data).await.unwrap();
    if let Some(result) = response[0].as_str() {
        assert_eq!("ok", result);
    } else {
        panic!("can't receive response from node");
    }
}


#[tokio::test]
async fn test_add_module_mounted_binaries_forbidden() {
    let swarms = make_swarms(1).await;

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .expect("connect client");
    let module = load_module("tests/tetraplets/artifacts", "tetraplets").expect("load module");

    let config = json!(
        {
            "name": "tetraplets",
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            },
            "mounted_binaries": json!({"cmd": "/usr/bin/behbehbeh"})
        });

   let script = r#"
    (xor
       (seq
           (call node ("dist" "add_module") [module_bytes module_config])
           (call client ("return" "") ["ok"])
       )
       (call client ("return" "") [%last_error%.$.message])
    )
   "#;

    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64.encode(&module)),
        "module_config" => config,
    };

    let response = client.execute_particle(script, data).await.unwrap();
    if let Some(result) = response[0].as_str() {
        let expected = "Local service error, ret_code is 1, error message is '\"Error: Config error: requested mounted binary /usr/bin/behbehbeh is forbidden on this host\\nForbiddenMountedBinary { forbidden_path: \\\"/usr/bin/behbehbeh\\\" }\"'";
        assert_eq!(expected, result);
    } else {
        panic!("can't receive response from node");
    }
}
