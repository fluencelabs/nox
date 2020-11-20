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

use crate::{test_module, test_module_cfg, ConnectedClient};
use maplit::hashmap;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct CreatedService {
    pub id: String,
}

pub fn create_greeting_service(client: &mut ConnectedClient) -> CreatedService {
    let module = "greeting";

    let script = f!(r#"(seq
            (call relay ("dist" "add_module") [module_bytes module_config] module)
            (seq
                (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                (seq
                    (call relay ("srv" "create") [blueprint_id] service_id)
                    (call client ("return" "") [service_id] client_result)
                )
            )
        )"#);
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(test_module())),
        "module_config" => test_module_cfg(module),
        "blueprint" => json!({ "name": "blueprint", "dependencies": [module] }),
    };

    client.send_particle(script, data);
    let response = client.receive_args();

    let service_id = response[0].as_str().expect("service_id").to_string();

    CreatedService { id: service_id }
}
