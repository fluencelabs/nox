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

// history service:
//      history.wasm
//      sqlite.wasm
// API:
//      fn add(author: String, msg: String) -> String
//      fn get_all() -> String
//      fn get_last(last: u64) -> String

// members service:
//      members.wasm
//      sqlite.wasm
// API:
//      fn init()
//      fn user_exists(user: &str) -> bool
//      fn update_name(user: String, name: String) -> String
//      fn update_relay(user: String, relay: String, sig: String) -> String
//      fn get_all_users() -> String
//      fn add_user(user: String, relay: String, sig: String, name: String) -> String
//      fn delete_user(user: &str) -> String

use config_utils::to_abs_path;
use json_utils::into_array;
use test_utils::{enable_logs, make_swarms, ConnectedClient, KAD_TIMEOUT};

use fstrings::format_f;
use libp2p::PeerId;
use particle_providers::Provider;
use serde_json::{json, Value as JValue};
use std::collections::HashSet;
use std::path::PathBuf;
use std::thread::sleep;

fn load_module(name: &str) -> Vec<u8> {
    let module = to_abs_path(PathBuf::from("tests/chat/").join(name));
    let module = std::fs::read(&module).expect(format!("fs::read from {:?}", module).as_str());

    module
}

fn module_config(module: &str) -> JValue {
    json!(
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
    )
}

fn create_service(client: &mut ConnectedClient, module: &str) -> String {
    let script = format!(
        r#"
        (seq (
            (seq (
                (call (%current_peer_id% ("add_module" "") (module_bytes module_config) void[]))
                (call (%current_peer_id% ("add_module" "") (sqlite_bytes sqlite_config) void[]))
            ))
            (seq (
                (call (%current_peer_id% ("add_blueprint" "") (blueprint) blueprint_id))
                (seq (
                    (call (%current_peer_id% ("create" "") (blueprint_id) service_id))
                    (call ("{}" ("" "") ("service_id") client_result))
                ))
            ))
        ))
        "#,
        client.peer_id
    );

    let data = json!({
        "module_bytes": base64::encode(load_module(format!("{}.wasm", module).as_str())),
        "module_config": module_config(module),
        "sqlite_bytes": base64::encode(load_module("sqlite.wasm")),
        "sqlite_config": module_config("sqlite"),
        "blueprint": { "name": module, "dependencies": ["sqlite", module] },
    });

    client.send_particle(script, data);
    let response = client.receive();

    response.data["service_id"]
        .as_str()
        .expect("missing service_id")
        .to_string()
}

fn alias_service(name: &str, node: PeerId, service_id: String, client: &mut ConnectedClient) {
    let name = bs58::encode(name).into_string();
    let script = format_f!(
        r#"
        (seq (
            (call ("{node}" ("neighborhood" "") ("{name}") neighbors))
            (fold (neighbors n
                (seq (
                    (call (n ("add_provider" "") ("{name}" provider) void[]))
                    (next n)
                ))
            ))
        ))
        "#
    );
    let provider = Provider {
        peer: node,
        service_id: Some(service_id),
    };
    client.send_particle(script, json!({ "provider": provider }));
}

fn resolve_service(name: &str, node: PeerId, client: &mut ConnectedClient) -> HashSet<Provider> {
    let name = bs58::encode(name).into_string();
    let script = format_f!(
        r#"
        (seq (
            (seq (
                (call ("{node}" ("neighborhood" "") ("{name}") neighbors))
                (fold (neighbors n
                    (seq (
                        (call (n ("get_providers" "") ("{name}") providers[]))
                        (next n)
                    ))
                ))
            ))
            (seq (
                (call ("{client.node}" ("identity" "") () void[]))
                (call ("{client.peer_id}" ("identity" "") (providers) void[]))
            ))
        ))
    "#
    );

    client.send_particle(script, json!({}));
    let response = client.receive();
    let providers = into_array(response.data["providers"].clone())
        .expect("missing providers")
        .into_iter()
        .filter_map(|p| {
            let p = into_array(p)?[0].clone();
            serde_json::from_value::<Provider>(p.clone())
                .expect(format!("deserialize provider: {:#?}", p).as_str())
                .into()
        })
        .collect();

    providers
}

/*fn call_service(service: &str, fname: &str, client: &mut ConnectedClient) {
    let script = f!(
        r#"
            (seq (
                (call ("{}" ("{}" "{}"))
            ))
        "#,
        node, service, fname
    );
}*/

fn create_history(client: &mut ConnectedClient) -> String {
    create_service(client, "history")
}
fn create_members(client: &mut ConnectedClient) -> String {
    create_service(client, "members")
}
fn join_chat() {}
fn send_message() {}
fn get_history() {}
fn get_members() {}

#[test]
fn test_chat() {
    enable_logs();

    let swarms = make_swarms(5);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let history = create_history(&mut client);
    let members = create_members(&mut client);

    println!("{} {}", history, members);

    alias_service("history", client.node.clone(), history, &mut client);
    let providers = resolve_service("history", client.node.clone(), &mut client);
    println!("{:#?}", providers);
}
