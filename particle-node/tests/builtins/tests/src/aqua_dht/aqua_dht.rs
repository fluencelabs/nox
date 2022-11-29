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

use super::Record;
use crate::{load_script, SERVICES, SPELL};
use connected_client::ConnectedClient;
use created_swarm::make_swarms_with_builtins;

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;

#[test]
fn put_get() {
    let register_put_script = load_script("dht-api.registerKeyPutValue.air");
    let get_values_script = load_script("dht-api.getValues.air");

    let swarms = make_swarms_with_builtins(3, SERVICES.as_ref(), None, Some(SPELL.to_string()));

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

    assert!(records.len() > 0);

    for record in records {
        assert_eq!(&record.value, "value");
        assert_eq!(record.service_id, vec![String::from("service_id")]);
    }
}
