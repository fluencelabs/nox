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
//      sqlite3.wasm
// API:
//      fn add(author: String, msg: String) -> String
//      fn get_all() -> String
//      fn get_last(last: u64) -> String

// user-list service:
//      user-list.wasm
//      sqlite3.wasm
// API:
//      fn join(user: String, relay: String, sig: String, name: String) -> String
//      fn get_users() -> Vec<User>
//      fn change_name(user: String, name: String, signature: String) -> String
//      fn change_relay(user: String, relay: String, sig: String, signature: String) -> String
//      fn delete(user: String, signature: String) -> String
//      fn is_exists(user: String) -> bool

use json_utils::into_array;
use particle_providers::Provider;
use test_utils::{connect_swarms, load_module, module_config, ConnectedClient};

use eyre::{ContextCompat, WrapErr};
use fstrings::f;
use itertools::Itertools;
use libp2p::PeerId;
use maplit::hashmap;
use serde_json::{json, Value as JValue};
use std::collections::HashSet;

fn create_service(client: &mut ConnectedClient, module: &str) -> String {
    let script = r#"
        (seq
            (seq
                (call node ("dist" "add_module") [module_bytes module_config])
                (call node ("dist" "add_module") [sqlite_bytes sqlite_config])
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

    let module_bytes = load_module("tests/chat/", module);
    let module_hash = format!("hash:{}", blake3::hash(&module_bytes).to_hex().as_str());
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(module_bytes)),
        "module_config" => json!(module_config(module)),
        "sqlite_bytes" => json!(base64::encode(load_module("tests/chat/", "sqlite3"))),
        "sqlite_config" => json!(module_config("sqlite3")),
        "blueprint" => json!({ "name": module, "dependencies": ["name:sqlite3", module_hash] }),
    };

    client.send_particle(script, data);
    let response = client.receive_args().wrap_err("receive").unwrap();

    response[0]
        .as_str()
        .wrap_err("missing service_id")
        .unwrap()
        .to_string()
}

fn alias_service(name: &str, node: PeerId, service_id: String, client: &mut ConnectedClient) {
    let name = bs58::encode(name).into_string();
    let script = f!(r#"
        (seq
            (seq
                (call node ("kad" "neighborhood") ["{name}"] neighbors)
                (fold neighbors n
                    (seq
                        (call n ("deprecated" "add_provider") ["{name}" provider])
                        (next n)
                    )
                )
            )
            (seq
                (call node ("op" "identity") [])
                (call client ("return" "") [provider] client_result)
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
            "client" => json!(client.peer_id.to_string()),
        },
    );

    client.receive_args().wrap_err("receive").unwrap();
}

fn resolve_service(orig_name: &str, client: &mut ConnectedClient) -> HashSet<Provider> {
    let name = bs58::encode(orig_name).into_string();
    let script = f!(r#"
        (seq
            (seq
                (call node ("kad" "neighborhood") ["{name}"] neighbors)
                (fold neighbors n
                    (seq
                        (call n ("deprecated" "get_providers") ["{name}"] providers_{orig_name}[])
                        (next n)
                    )
                )
            )
            (seq
                (call node ("op" "identity") [])
                (call client ("return" "") [providers_{orig_name}])
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
    let response = client.receive_args().wrap_err("receive").unwrap();
    println!("resolve_service {} respoonse: {:#?}", orig_name, response);
    let providers = into_array(response[0].clone())
        .wrap_err(format!("missing providers: {:#?}", response))
        .unwrap()
        .into_iter()
        .filter_map(|p| {
            let p = into_array(p)?[0].clone();
            serde_json::from_value::<Provider>(p.clone())
                .wrap_err(format!("deserialize provider: {:#?}", p))
                .unwrap()
                .into()
        })
        .collect();

    providers
}

#[rustfmt::skip]
fn call_service(alias: &str, fname: &str, args: &[(&'static str, JValue)], client: &mut ConnectedClient) -> JValue {
    let provider = resolve_service(alias, client).into_iter().next().wrap_err(f!("no providers found for {alias}")).unwrap();
    let service_id = provider.service_id.wrap_err("get service id").unwrap();

    let arg_names = args.iter().map(|(name, _)| *name).join(" ");
    let script = f!(r#"
        (seq
            (seq
                (call node ("op" "identity") [])
                (call provider (service_id "{fname}") [{arg_names}] result)
            )
            (seq
                (call node ("op" "identity") [])
                (call client ("return" "") [result])
            )
        )
    "#);
    
    let mut data = hashmap! {
        "provider" => json!(provider.peer.to_string()),
        "service_id" => json!(service_id),
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
    }; 
    let args = args.iter().map(|t| t.clone());
    data.extend(args);
    
    client.send_particle(script, data);

    client.receive_args().wrap_err("receive args").unwrap()[0].take()
}

fn create_history(client: &mut ConnectedClient) -> String {
    create_service(client, "history")
}
fn create_userlist(client: &mut ConnectedClient) -> String {
    create_service(client, "user-list")
}
fn join_chat(name: String, client: &mut ConnectedClient) {
    call_service(
        "user-list",
        "join",
        &[(
            "user",
            json!({
                "peer_id": client.peer_id.to_string(),
                "relay_id": client.node.to_string(),
                "signature": client.peer_id.to_string(),
                "name": name,
            }),
        )],
        client,
    );
}

fn get_users(client: &mut ConnectedClient) -> Vec<JValue> {
    into_array(
        call_service("user-list", "get_users", &[], client)
            .as_object_mut()
            .unwrap()
            .remove("users")
            .unwrap(),
    )
    .wrap_err("user list must be an array")
    .unwrap()
}

fn get_history(client: &mut ConnectedClient) -> Vec<JValue> {
    let mut response = call_service("history", "get_all", &[], client);
    #[rustfmt::skip]
    let history = response.as_object_mut().unwrap().remove("messages").unwrap();
    let history = into_array(history)
        .wrap_err("history must be an array")
        .unwrap();

    history
}

#[rustfmt::skip]
fn send_message(msg: &str, author: &str, client: &mut ConnectedClient) {
    let history = resolve_service("history", client).into_iter().next().wrap_err("no providers found").unwrap();
    let history_id = history.service_id.wrap_err("get service id").unwrap();
    let userlist = resolve_service("user-list", client).into_iter().next().wrap_err("no providers found").unwrap();
    let userlist_id = userlist.service_id.wrap_err("get service id").unwrap();
    
    // user = [0]
    // relay = [1]
    let script = f!(r#"
        (seq
            (seq
                (call node ("op" "identity") [])
                (seq
                    (call history (history_id "add") [author msg zero])
                    (call userlist (userlist_id "get_users") [] users)
                )
            )
            (fold users.$.users! u
                (par 
                    (seq
                        (call u.$["relay_id"]! ("op" "identity") [])
                        (call u.$["peer_id"]! ("receive" "") [msg])
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
        "zero" => json!(0)
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

    call_service(
        "history",
        "add",
        &[
            ("author", json!("author")),
            ("msg", json!("message one")),
            ("zero", json!(0)),
        ],
        &mut client,
    );
    call_service(
        "history",
        "add",
        &[
            ("author", json!("author")),
            ("msg", json!("message 2")),
            ("zero", json!(0)),
        ],
        &mut client,
    );

    let history = get_history(&mut client);
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
    client.receive().wrap_err("receive").unwrap();
    for c in clients.iter_mut() {
        c.receive().wrap_err("receive").unwrap();
    }
    let history = get_history(&mut client);
    assert_eq!(3, history.len());

    join_chat("фолекс".to_string(), &mut client);
    join_chat("шмолекс".to_string(), &mut client);
    join_chat("кролекс".to_string(), &mut client);
    assert_eq!(1 + node_count, get_users(&mut client).len());
}
