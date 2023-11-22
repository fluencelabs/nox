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

use eyre::WrapErr;
use itertools::Itertools;
use libp2p::PeerId;
use maplit::hashmap;
use serde_json::{json, Value as JValue};

use connected_client::ConnectedClient;
use created_swarm::make_swarms;
use log_utils::enable_logs;
use particle_protocol::Contact;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn neighborhood_heavy() {
    enable_logs();
    let swarms = make_swarms(3).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let response = client
        .execute_particle(
            r#"
            (seq
                (call node ("kad" "neighborhood") [node] peers)
                (call client ("return" "") [peers] void)
            )
        "#,
            hashmap! {
                "node" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string())
            },
        )
        .await
        .unwrap();
    if let JValue::Array(neighborhood) = response[0].clone() {
        assert_eq!(neighborhood.len(), 2);

        let assert_contains = |id: &PeerId| {
            assert!(neighborhood
                .iter()
                .any(|v| *v == JValue::String(id.to_string())))
        };

        assert_contains(&swarms[1].peer_id);
        assert_contains(&swarms[2].peer_id);
    } else {
        panic!("response[0] must be an array, response was {:#?}", response);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn neighborhood_with_addresses_heavy() {
    enable_logs();
    let swarms = make_swarms(3).await;
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .await
        .wrap_err("connect client")
        .unwrap();

    let response = client
        .execute_particle(
            r#"
            (seq
                (call node ("kad" "neigh_with_addrs") [node] peers)
                (call client ("return" "") [peers] void)
            )
        "#,
            hashmap! {
                "node" => json!(client.node.to_string()),
                "client" => json!(client.peer_id.to_string())
            },
        )
        .await
        .unwrap();
    let neighborhood = response.into_iter().next().expect("empty response");
    let neighborhood: Vec<Contact> =
        serde_json::from_value(neighborhood).expect("deserialize neighborhood");
    tracing::info!("neighborhood {:?}", neighborhood);
    assert_eq!(neighborhood.len(), 2);

    let first = neighborhood
        .iter()
        .find(|c| c.peer_id == swarms[1].peer_id)
        .expect("1st node wasn't found in neighborhood");
    assert!(
        first.addresses.iter().contains(&swarms[1].multiaddr),
        "1st node's multiaddr not found in contact"
    );

    let second = neighborhood
        .iter()
        .find(|c| c.peer_id == swarms[2].peer_id)
        .expect("2nd node wasn't found in neighborhood");
    assert!(
        second.addresses.iter().contains(&swarms[2].multiaddr),
        "2nd node's multiaddr not found in contact"
    );
}
