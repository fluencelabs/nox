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

// user-list service:
//      user-list.wasm
//      sqlite.wasm
// API:
//      fn join(user: String, relay: String, sig: String, name: String) -> String
//      fn get_users() -> Vec<User>
//      fn change_name(user: String, name: String, signature: String) -> String
//      fn change_relay(user: String, relay: String, sig: String, signature: String) -> String
//      fn delete(user: String, signature: String) -> String
//      fn is_exists(user: String) -> bool

use config_utils::to_abs_path;
use json_utils::into_array;
use particle_providers::Provider;
use test_utils::{connect_swarms, ConnectedClient};

use fstrings::f;
use libp2p::PeerId;
use maplit::hashmap;
use serde_json::{json, Value as JValue};
use std::{collections::HashSet, path::PathBuf};

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
    let script = r#"
        (seq
            (seq
                (call node ("dist" "add_module") [module_bytes module_config] void[])
                (call node ("dist" "add_module") [sqlite_bytes sqlite_config] void[])
            )
            (seq
                (call node ("dist" "add_blueprint") [blueprint] blueprint_id)
                (seq
                    (call node ("srv" "create") [blueprint_id] service_id)
                    (call client ("return" "") [service_id] client_result)
                )
            )
        )
        "#;
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(load_module(format!("{}.wasm", module).as_str()))),
        "module_config" => json!(module_config(module)),
        "sqlite_bytes" => json!(base64::encode(load_module("sqlite.wasm"))),
        "sqlite_config" => json!(module_config("sqlite")),
        "blueprint" => json!({ "name": module, "dependencies": ["sqlite", module] }),
    };

    client.send_particle(script, data);
    let response = client.receive_args();

    response[0]
        .as_str()
        .expect("missing service_id")
        .to_string()
}

fn alias_service(name: &str, node: PeerId, service_id: String, client: &mut ConnectedClient) {
    let name = bs58::encode(name).into_string();
    let script = f!(r#"
        (seq
            (call node ("dht" "neighborhood") ["{name}"] neighbors)
            (fold neighbors n
                (seq
                    (call n ("dht" "add_provider") ["{name}" provider] void[])
                    (next n)
                )
            )
        )
        "#);
    let provider = Provider {
        peer: node,
        service_id: Some(service_id),
    };
    client.send_particle(
        script,
        hashmap! {
            "provider" => json!(provider),
            "node" => json!(client.node.to_string()),
        },
    );
}

fn resolve_service(orig_name: &str, client: &mut ConnectedClient) -> HashSet<Provider> {
    let name = bs58::encode(orig_name).into_string();
    let script = f!(r#"
        (seq
            (seq
                (call node ("dht" "neighborhood") ["{name}"] neighbors)
                (fold neighbors n
                    (seq
                        (call n ("dht" "get_providers") ["{name}"] providers_{orig_name}[])
                        (next n)
                    )
                )
            )
            (seq
                (call node ("op" "identity") [] void[])
                (call client ("return" "") [providers_{orig_name}] void[])
            )
        )
    "#);

    client.send_particle(
        script,
        hashmap! {
            "client" => json!(client.peer_id.to_string()),
            "node" => json!(client.node.to_string()),
        },
    );
    let response = client.receive_args();
    log::info!("resolve_service {} respoonse: {:#?}", orig_name, response);
    let providers = into_array(response[0].clone())
        .expect(format!("missing providers: {:#?}", response).as_str())
        .into_iter()
        // .expect(format!("missing providers: {:#?}", response).as_str())
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

#[rustfmt::skip]
fn call_service(alias: &str, fname: &str, arg_list: &str, client: &mut ConnectedClient) -> JValue {
    let provider = resolve_service(alias, client).into_iter().next().expect(f!("no providers found for {alias}").as_str());
    let service_id = provider.service_id.expect("get service id");

    let script = f!(r#"
        (seq
            (seq
                (call node ("op" "identity") [] void[])
                (call provider (service_id "{fname}") {arg_list} result)
            )
            (seq
                (call node ("op" "identity") [] void[])
                (call client ("return" "") [result] void[])
            )
        )
    "#);
    client.send_particle(script, hashmap! {
        "provider" => json!(provider.peer.to_string()),
        "service_id" => json!(service_id),
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
    });

    client.receive_args()[0].take()
}

fn create_history(client: &mut ConnectedClient) -> String {
    create_service(client, "history")
}
fn create_userlist(client: &mut ConnectedClient) -> String {
    create_service(client, "user-list")
}
fn join_chat(name: String, client: &mut ConnectedClient) {
    let sig = &client.peer_id;
    call_service(
        "user-list",
        "join",
        f!(r#"["{client.peer_id}" "{client.node}" "{sig}" "{name}"]"#).as_str(),
        client,
    );
}

fn get_users(client: &mut ConnectedClient) -> Vec<JValue> {
    into_array(call_service(
        "user-list",
        "get_users",
        f!(r"[]").as_str(),
        client,
    ))
    .expect("user list must be an array")
}

#[rustfmt::skip]
fn send_message(msg: &str, author: &str, client: &mut ConnectedClient) {
    let history = resolve_service("history", client).into_iter().next().expect("no providers found");
    let history_id = history.service_id.expect("get service id");
    let userlist = resolve_service("user-list", client).into_iter().next().expect("no providers found");
    let userlist_id = userlist.service_id.expect("get service id");
    
    // user = [0]
    // relay = [1]
    let script = f!(r#"
        (seq
            (seq
                (call node ("op" "identity") [] void[])
                (seq
                    (call history (history_id "add") [author msg] void[])
                    (call userlist (userlist_id "get_users") [] users)
                )
            )
            (fold users u
                (par 
                    (seq
                        (call u.$["relay_id"] ("op" "identity") [] void[])
                        (call u.$["peer_id"] ("receive" "") [msg] void[])
                    ) 
                    (next u)
                )
            )
        )
    "#);
    client.send_particle(script, hashmap!{
        "history" => json!(history.peer.to_string()),
        "history_id" => json!(history_id),
        "userlist" => json!(userlist.peer.to_string()),
        "userlist_id" => json!(userlist_id),
        "author" => json!(author),
        "msg" => json!(msg),
        "node" => json!(client.node.to_string()),
    });
}

#[test]
fn test_chat() {
    let node_count = 5;
    let connect = connect_swarms(node_count);
    let mut client = connect(0);

    let history = create_history(&mut client);
    let userlist = create_userlist(&mut client);

    alias_service("history", client.node.clone(), history, &mut client);
    assert!(!resolve_service("history", &mut client).is_empty());

    call_service("history", "add", r#"["author" "msg1"]"#, &mut client);
    call_service("history", "add", r#"["author" "msg2"]"#, &mut client);

    let history = call_service("history", "get_all", "[]", &mut client);
    let history = into_array(history).expect("history must be an array");
    assert_eq!(2, history.len());

    alias_service("user-list", client.node.clone(), userlist, &mut client);
    assert!(!resolve_service("user-list", &mut client).is_empty());

    join_chat("кекекс".to_string(), &mut client);
    assert_eq!(1, get_users(&mut client).len());

    log::info!("Adding vovans");
    let mut clients: Vec<_> = (0..node_count).map(|i| connect(i)).collect();
    for (i, c) in clients.iter_mut().enumerate() {
        log::info!("Adding vovan {}", i);
        join_chat(f!("vovan{i}"), c);
        log::info!("Vovan added {}", i);
    }
    log::info!("Added all vovans");
    assert_eq!(1 + node_count, get_users(&mut client).len());

    send_message(r#"привет\ вованы"#, r#"главный\ Вован🤡"#, &mut client);
    client.receive();
    for c in clients.iter_mut() {
        c.receive();
    }
    let history = call_service("history", "get_all", "[]", &mut client);
    let history = into_array(history).expect("history must be an array");
    assert_eq!(3, history.len());

    join_chat("фолекс".to_string(), &mut client);
    join_chat("шмолекс".to_string(), &mut client);
    join_chat("кролекс".to_string(), &mut client);
    assert_eq!(1 + node_count, get_users(&mut client).len());
}
