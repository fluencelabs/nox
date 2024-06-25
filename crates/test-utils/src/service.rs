/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use maplit::hashmap;
use serde_json::json;

use connected_client::ConnectedClient;
use service_modules::Hash;

#[derive(Debug, Clone)]
pub struct CreatedService {
    pub id: String,
}

pub async fn create_service(
    client: &mut ConnectedClient,
    module_name: &str,
    module_bytes: Vec<u8>,
) -> CreatedService {
    create_service_worker(client, module_name, module_bytes, client.node.to_string()).await
}

pub async fn create_service_worker(
    client: &mut ConnectedClient,
    module_name: &str,
    module_bytes: Vec<u8>,
    worker_id: String,
) -> CreatedService {
    let script = f!(r#"
    (seq
        (seq
            (call relay ("dist" "default_module_config") [module_name] module_config)
            (call relay ("dist" "add_module") [module_bytes module_config] module)
        )
        (seq
            (seq
                (call relay ("dist" "make_blueprint") [name dependencies] blueprint)
                (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
            )
            (seq
                (call worker_id ("srv" "create") [blueprint_id] service_id)
                (call client ("return" "") [service_id] client_result)
            )
        )
    )
    "#);

    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "relay" => json!(client.node.to_string()),
        "worker_id" => json!(worker_id),
        "module_name" => json!(module_name),
        "dependencies" => json!([Hash::new(&module_bytes).unwrap()]),
        "module_bytes" => json!(base64.encode(module_bytes)),
        "name" => json!("blueprint"),
    };

    let response = client.execute_particle(script, data).await.unwrap();

    let service_id = response[0]
        .as_str()
        .expect("service_id is in response")
        .to_string();

    CreatedService { id: service_id }
}
