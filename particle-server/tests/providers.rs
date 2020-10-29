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

use fluence_libp2p::RandomPeerId;
use json_utils::into_array;
use particle_providers::Provider;
use test_utils::{enable_logs, make_swarms_with_cfg, uuid, ConnectedClient, KAD_TIMEOUT};

use maplit::hashmap;
use serde_json::json;
use std::{collections::HashSet, thread::sleep};

#[test]
fn add_providers() {
    enable_logs();

    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let provider1 = "provider1";
    let provider2 = "provider2";
    client.send_particle(
        r#"
        (seq (
            (call (node ("add_provider" "") (key provider) void[]))
            (seq (
                (call (node ("add_provider" "") (key2 provider2) void[]))
                (seq (
                    (call (node ("get_providers" "") (key2) providers[]))
                    (seq (
                        (call (node ("get_providers" "") (key) providers[]))
                        (seq (
                            (call (node2 ("identity" "") () void[]))
                            (call (client2 ("return" "") (providers[]) void[]))
                        ))
                    ))
                ))
            ))
        ))
        "#,
        hashmap!{
            "client" => json!(client.peer_id.to_string()),
            "node" => json!(client.node.to_string()),
            "client2" => json!(client2.peer_id.to_string()),
            "node2" => json!(client2.node.to_string()),
            "provider" => json!({"peer": RandomPeerId::random().to_string(), "service_id": provider1}),
            "key" => json!("folex"),
            "provider2" => json!({"peer": RandomPeerId::random().to_string(), "service_id": provider2}),
            "key2" => json!("folex2"),
        },
    );

    let particle = client2.receive_args();
    let providers = particle[0].as_array().expect("non empty providers");
    println!("providers: {:#?}", providers);
    assert_eq!(providers.len(), 2);
    #[rustfmt::skip]
    let get_provider = |i: usize| {
        providers
            .get(i).unwrap()
            .as_array().unwrap()
            .get(0).unwrap()
            .get("service_id").unwrap()
            .as_str().unwrap()
    };
    assert_eq!(get_provider(0), provider1);
    assert_eq!(get_provider(1), provider2);
}

#[test]
fn add_providers_to_neighborhood() {
    let swarms = make_swarms_with_cfg(10, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let provider1 = uuid();
    // TODO: add two more folds (for provider2), and this test will time out. Investigate reasons and fix.
    let script = r#"
    (seq (
        (call (node ("neighborhood" "") (first_node) neighborhood))
        (seq (
            (seq (
                (fold (neighborhood i
                    (par (
                        (call (i ("add_provider" "") (key provider) void[]))
                        (next i)
                    ))
                ))
                (fold (neighborhood i
                    (par (
                        (call (i ("get_providers" "") (key) providers[]))
                        (next i)
                    ))
                ))
            ))
            (seq (
                (call (node ("identity" "") () void[]))
                (call (client ("" "") () none))
            ))
        ))
    ))
        "#;

    let provider = Provider {
        peer: RandomPeerId::random(),
        service_id: provider1.into(),
    };
    client.send_particle(
        script,
        hashmap! {
            "client" => json!(client.peer_id.to_string()),
            "node" => json!(client.node.to_string()),
            "provider" => json!(provider),
            "key" => json!("folex"),
            "first_node" => json!(swarms[0].0.to_string()),
        },
    );

    let response = client2.receive();
    let providers = into_array(response.data["providers"].clone().take())
        .expect(format!("providers must be array, data was {:#?}", response.data).as_str());
    let providers: Vec<_> = providers
        .into_iter()
        .flat_map(|v| into_array(v).expect("must be array"))
        .map(|v| serde_json::from_value::<Provider>(v).expect("be provider"))
        .collect();
    let providers: HashSet<_> = providers.into_iter().collect();
    assert_eq!(providers.len(), 1);
    assert!(providers.contains(&provider));
    // assert!(providers.contains(&provider2));
}
