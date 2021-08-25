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

use crate::aqua_dht::Record;
use crate::load_script;
use connected_client::ConnectedClient;
use created_swarm::make_swarms_with_builtins;

use eyre::WrapErr;
use maplit::hashmap;
use serde_json::json;
use std::path::Path;

#[test]
fn find_subscribers() {
    let init_subscribe_script = load_script("pubsub.initTopicAndSubscribe.air");
    let find_subscribers_script = load_script("pubsub.findSubscribers.air");

    let swarms = make_swarms_with_builtins(3, Path::new("../deploy/builtins"), None);

    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    // initTopicAndSubscribe(node_id: PeerId, topic: string, value: string, relay_id: ?PeerId, service_id: ?string):
    let topic = "topic";
    let init_particle = client.send_particle_ext(
        init_subscribe_script,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "topic" => json!(topic),
            "value" => json!("value"),
            // relay_id & service_id are of Option type, so they are represented as arrays
            "relay_id" => json!([client.node.to_string()]),
            "service_id" => json!(["service_id"]),
        },
        true,
    );
    client
        .wait_particle_args(&init_particle)
        .expect("execute initTopicAndSubscribe");

    // func findSubscribers(node_id: PeerId, topic: string) -> []Record:
    let find_subscribers_particle = client.send_particle_ext(
        find_subscribers_script,
        hashmap! {
            "-relay-" => json!(client.node.to_string()),
            "node_id" => json!(client.node.to_string()),
            "topic" => json!(topic),
        },
        true,
    );
    let mut result = client
        .wait_particle_args(&find_subscribers_particle)
        .expect("execute findSubscribers");
    let records = result.get_mut(0).unwrap().take();
    let records: Vec<Record> = serde_json::from_value(records).unwrap();

    for record in records {
        assert_eq!(&record.value, "value");
        assert_eq!(record.service_id, vec![String::from("service_id")]);
    }
}
