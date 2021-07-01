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

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use eyre::{ContextCompat, WrapErr};
use maplit::hashmap;
use serde_json::json;
use service_modules::{load_module, module_config};

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

    let module_bytes = load_module("tests/aqua_dht/", module).expect("load aqua-dht wasm");
    let module_hash = format!("hash:{}", blake3::hash(&module_bytes).to_hex().as_str());
    let sqlite_module = load_module("tests/aqua_dht/", "sqlite3").expect("load sqlite wasm");
    let data = hashmap! {
        "client" => json!(client.peer_id.to_string()),
        "node" => json!(client.node.to_string()),
        "module_bytes" => json!(base64::encode(module_bytes)),
        "module_config" => json!(module_config(module)),
        "sqlite_bytes" => json!(base64::encode(sqlite_module)),
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
    let register_put_script = load_script("dht-api.registerKeyPutValue.air");
    let get_values_script = load_script("dht-api.getValues.air");

    let swarms = make_swarms(3);

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
        client.receive_args().unwrap();
    }

    let key = "key";
    let put_particle = client.send_particle_ext(
        register_put_script,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "key" => json!(key),
            "value" => json!("value"),
            // relay_id & service_id are of Option type, so they are represented as arrays
            "relay_id" => json!([client.node.to_string()]),
            "service_id" => json!(["service_id"]),
        },
        true,
    );
    client
        .wait_particle_args(&put_particle)
        .expect("execute registerKeyPutValue");

    let get_particle = client.send_particle_ext(
        get_values_script,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "relay_id" => json!(client.node.to_string()),
            "key" => json!(key),
        },
        true,
    );
    let mut result = client
        .wait_particle_args(&get_particle)
        .expect("execute getValues");
    let records = result.get_mut(0).unwrap().take();
    let records: Vec<Record> = serde_json::from_value(records).unwrap();

    for record in records {
        assert_eq!(&record.value, "value");
        assert_eq!(record.service_id, vec![String::from("service_id")]);
    }
}

#[derive(serde::Deserialize, Debug)]
struct Record {
    value: String,
    peer_id: String,
    set_by: String,
    relay_id: Vec<String>,
    service_id: Vec<String>,
    timestamp_created: u64,
    weight: u32,
}

/*

data Record:
    value: string
    peer_id: string
    set_by: string
    relay_id: []string
    service_id: []string
    timestamp_created: u64
    weight: u32

   {
       "peer_id": "12D3KooWFoLti25GytKkEaZCbAgXFdHr3EBYmAxzbwf4fvLTBGEq",
       "relay_id": [
           "12D3KooWDHXBHVmyhxGbcQWAYFaeHkYdHTVcSdV9bwFEJZRdk8K7"
       ],
       "service_id": [
           "service_id"
       ],
       "set_by": "12D3KooWFoLti25GytKkEaZCbAgXFdHr3EBYmAxzbwf4fvLTBGEq",
       "timestamp_created": 1625147497,
       "value": "value",
       "weight": 0
   },
*/
