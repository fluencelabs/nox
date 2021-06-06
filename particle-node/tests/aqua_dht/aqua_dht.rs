/*
 * Copyright 2021 Fluence Labs Limited
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

use eyre::{ContextCompat, WrapErr};
use maplit::hashmap;
use serde_json::json;
use std::time::Duration;
use test_utils::{enable_logs, load_module, make_swarms, module_config, ConnectedClient};

fn load_script(name: &str) -> String {
    std::fs::read_to_string(format!("./tests/aqua_dht/aqua/{}", name)).unwrap()
}

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

    let module_bytes = load_module("tests/aqua_dht/", module);
    let module_hash = format!("hash:{}", blake3::hash(&module_bytes).to_hex().as_str());
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(module_bytes)),
        "module_config" => json!(module_config(module)),
        "sqlite_bytes" => json!(base64::encode(load_module("tests/aqua_dht/", "sqlite3"))),
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

#[test]
fn put_get() {
    let put_script = load_script("dht-api.registerKeyPutValue.air");
    let get_values = load_script("dht-api.getValues.air");

    let swarms = make_swarms(3);

    enable_logs();

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    for s in swarms {
        let mut client = ConnectedClient::connect_with_keypair(
            s.multiaddr.clone(),
            Some(s.management_keypair.clone()),
        )
        .unwrap();
        let dht_service = create_service(&mut client, "aqua-dht");
        client.send_particle(
            r#"
            (seq
                (call -relay- ("srv" "add_alias") [alias service])
                (call %init_peer_id% ("op" "return") [])
            )
            "#,
            hashmap! {
                "-relay-" => json!(client.node.to_string()),
                "service" => json!(dht_service),
                "alias" => json!("aqua-dht"),
            },
        );
        client.receive_args();
    }

    let key = "key";
    let id = dbg!(client.send_particle_ext(
        put_script,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "relay_id" => json!(client.node.to_string()),
            "key" => json!(key),
            "value" => json!("value"),
            "service_id" => json!("service_id"),
        },
        true
    ));
    let result = client.receive_args_from(&id);
    println!("result: {:?}", result);
    std::thread::sleep(Duration::from_secs(1));

    let id = dbg!(client.send_particle_ext(
        get_values,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "relay_id" => json!(client.node.to_string()),
            "key" => json!(key),
        },
        true
    ));
    let result = client.receive_args_from(&id);
    println!("result: {:?}", result);
}
