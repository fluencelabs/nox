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
use created_swarm::make_swarms_with_builtins;

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;
use std::path::Path;

fn load_script(name: &str) -> String {
    std::fs::read_to_string(format!("./tests/aqua_dht/aqua/{}", name)).unwrap()
}

#[test]
fn put_get() {
    let register_put_script = load_script("dht-api.registerKeyPutValue.air");
    let get_values_script = load_script("dht-api.getValues.air");

    let swarms = make_swarms_with_builtins(3, Path::new("../deploy/builtins"), None);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

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
