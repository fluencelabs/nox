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
use test_utils::{enable_logs, make_swarms, ConnectedClient, KAD_TIMEOUT};

use fstrings::f;
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use particle_providers::Provider;
use serde_json::{json, Value as JValue};
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Duration, Instant};

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
    let now = Instant::now();
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

    log::info!("create_service took {:?}", now.elapsed());

    response.data["service_id"]
        .as_str()
        .expect("missing service_id")
        .to_string()
}

fn alias_service(name: &str, node: PeerId, service_id: String, client: &mut ConnectedClient) {
    let name = bs58::encode(name).into_string();
    let script = f!(r#"
        (seq (
            (call ("{client.node}" ("neighborhood" "") ("{name}") neighbors))
            (fold (neighbors n
                (seq (
                    (call (n ("add_provider" "") ("{name}" provider) void[]))
                    (next n)
                ))
            ))
        ))
        "#);
    let provider = Provider {
        peer: node,
        service_id: Some(service_id),
    };
    client.send_particle(script, json!({ "provider": provider }));
}

fn resolve_service(name: &str, client: &mut ConnectedClient) -> HashSet<Provider> {
    let name = bs58::encode(name).into_string();
    let script = f!(r#"
        (seq (
            (seq (
                (call ("{client.node}" ("neighborhood" "") ("{name}") neighbors))
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
    "#);

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

#[rustfmt::skip]
fn call_service(alias: &str, fname: &str, arg_list: &str, client: &mut ConnectedClient) -> JValue {
    let provider = resolve_service(alias, client).into_iter().next().expect("no providers found");
    let service_id = provider.service_id.expect("get service id");

    let script = f!(r#"
        (seq (
            (call ("{provider.peer}" ("{service_id}" "{fname}") {arg_list} result))
            (seq (
                (call ("{client.node}" ("identity" "") () void[]))
                (call ("{client.peer_id}" ("identity" "") (result) void[]))
            ))
        ))
    "#);
    client.send_particle(script, json!({}));

    client.receive().data["result"].take()
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
        f!(r#"("{client.peer_id}" "{client.node}" "{sig}" "{name}")"#).as_str(),
        client,
    );
}

#[rustfmt::skip]
fn send_message(msg: &str, client: &mut ConnectedClient) {
    let provider = resolve_service("user-list", client).into_iter().next().expect("no providers found");
    let service_id = provider.service_id.expect("get service id");
    
    // user = [0]
    // relay = [1]
    let script = f!(r#"
        (seq (
            (call ("{provider.peer}" ("{service_id}" "get_users") () users))
            (fold (users u
                (par (
                    (seq (
                        (call (u.$[1] ("identity" "") () void[]))
                        (call (u.$[0] ("receive" "") (|"{msg}"|) void[]))
                    )) 
                    (next u)
                ))
            ))
        ))
    "#);
    client.send_particle(script, json!({}));
}

fn connect_swarms(node_count: usize) -> impl Fn(usize) -> ConnectedClient {
    let swarms = make_swarms(node_count);
    sleep(KAD_TIMEOUT);
    sleep(Duration::from_secs(3));

    move |i| ConnectedClient::connect_to(swarms[i].1.clone()).expect("connect client")
}

fn connect_real(node_count: usize) -> impl Fn(usize) -> ConnectedClient {
    let nodes = vec![
        /* STAGE
        "/ip4/134.209.186.43/tcp/7001",
        "/ip4/134.209.186.43/tcp/7002",
        "/ip4/134.209.186.43/tcp/7003",
        "/ip4/134.209.186.43/tcp/7004",
        "/ip4/134.209.186.43/tcp/7005",
        "/ip4/134.209.186.43/tcp/7770",
        "/ip4/134.209.186.43/tcp/7100",
        */

        /* DEV */
        "/ip4/165.227.164.3/tcp/7001",
        "/ip4/165.227.164.3/tcp/7002",
        "/ip4/165.227.164.3/tcp/7003",
        "/ip4/165.227.164.3/tcp/7004",
        "/ip4/165.227.164.3/tcp/7005",
        "/ip4/165.227.164.3/tcp/7770",
        "/ip4/165.227.164.3/tcp/7100",
    ]
    .into_iter()
    .map(|addr| Multiaddr::from_str(addr).expect("valid multiaddr"))
    .cycle()
    .take(node_count)
    .collect::<Vec<_>>();

    move |i| ConnectedClient::connect_to(nodes[i].clone()).expect("connect client")
}

#[test]
fn test_chat() {
    enable_logs();

    let node_count = 5;

    let connect = connect_swarms(node_count);
    let mut client = connect(0);

    let history = create_history(&mut client);
    let userlist = create_userlist(&mut client);

    alias_service("history", client.node.clone(), history, &mut client);
    assert!(!resolve_service("history", &mut client).is_empty());

    call_service("history", "add", r#"("author" "msg1")"#, &mut client);
    call_service("history", "add", r#"("author" "msg2")"#, &mut client);

    let result = call_service("history", "get_all", "()", &mut client);

    alias_service("user-list", client.node.clone(), userlist, &mut client);
    assert!(!resolve_service("user-list", &mut client).is_empty());

    join_chat("folex".to_string(), &mut client);
    let result = call_service("user-list", "get_users", "()", &mut client);
    println!("get_users result: {:#?}", result);

    let mut clients: Vec<_> = (0..node_count).map(|i| connect(i)).collect();
    for (i, c) in clients.iter_mut().enumerate() {
        join_chat(f!("vovan{i}"), c);
    }

    let result = call_service("user-list", "get_users", "()", &mut client);
    println!("get_users result: {:#?}", result);

    send_message(r#"hello\ vovans"#, &mut client);
    for mut c in clients.iter_mut() {
        c.receive();
        println!("received");
    }
}
