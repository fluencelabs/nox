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

use crate::{test_module_cfg_map, ConnectedClient};
use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct CreatedService {
    pub id: String,
}

pub fn create_service(
    client: &mut ConnectedClient,
    module_name: &str,
    module_bytes: Vec<u8>,
) -> CreatedService {
    let script = f!(r#"
    (seq
        (seq
            (call relay ("dist" "make_module_config") [module_name mem_pages_count logger_enabled preopened_files envs mapped_dirs mounted_binaries logging_mask] module_config)
            (call relay ("dist" "add_module") [module_bytes module_config] module)
        )
        (seq
            (seq
                (call relay ("dist" "make_blueprint") [name dependencies] blueprint)
                (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
            )
            (seq
                (call relay ("srv" "create") [blueprint_id] service_id)
                (call client ("return" "") [service_id] client_result)
            )
        )
    )
    "#);

    let mut data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(module_bytes)),
        "name" => json!("blueprint"),
        "dependencies" => json!([module_name]),
    };
    data.extend(test_module_cfg_map(module_name).into_iter());

    client.send_particle(script, data);
    let response = client.receive_args().wrap_err("receive args").unwrap();

    let service_id = response[0]
        .as_str()
        .expect("service_id is in response")
        .to_string();

    CreatedService { id: service_id }
}
