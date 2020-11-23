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
use test_utils::{make_swarms_with_cfg, uuid, ConnectedClient, KAD_TIMEOUT};

use maplit::hashmap;
use serde_json::json;
use std::{collections::HashSet, thread::sleep};

#[test]
fn add_providers() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let provider1 = "provider1";
    let provider2 = "provider2";
    client.send_particle(
        r#"
        (seq
            (call node ("dht" "add_provider") [key provider] void[])
            (seq
                (call node ("dht" "add_provider") [key2 provider2] void[])
                (seq
                    (call node ("dht" "get_providers") [key2] providers[])
                    (seq
                        (call node ("dht" "get_providers") [key] providers[])
                        (seq
                            (call node2 ("op" "identity") [] void[])
                            (call client2 ("return" "") [providers] void[])
                        )
                    )
                )
            )
        )
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
    let find_provider = |service_id: &'static str| {
        providers
            .iter().find(|p| p.as_array().unwrap()[0]["service_id"].as_str().unwrap() == service_id)
    };
    assert!(find_provider(provider1).is_some());
    assert!(find_provider(provider2).is_some());
}

#[test]
fn add_providers_to_neighborhood() {
    let swarms = make_swarms_with_cfg(10, |cfg| cfg);

    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let script = r#"
    (seq
        (call node ("dht" "neighborhood") [first_node] neighborhood)
        (seq
            (seq
                (seq
                    (fold neighborhood i
                        (seq
                            (call i ("dht" "add_provider") [key provider] void[])
                            (next i)
                        )
                    )
                    (fold neighborhood i
                        (seq
                            (call i ("dht" "get_providers") [key] providers[])
                            (next i)
                        )
                    )
                )
                (seq
                    (fold neighborhood i
                        (seq
                            (call i ("dht" "add_provider") [key2 provider2] void[])
                            (next i)
                        )
                    )
                    (fold neighborhood i
                        (seq
                            (call i ("dht" "get_providers") [key2] providers[])
                            (next i)
                        )
                    )
                )
            )
            (seq
                (call node ("op" "identity") [] void[])
                (call client2 ("return" "") [providers] void[])
            )
        )
    )
    "#;

    let provider1 = uuid();
    let provider2 = uuid();

    let provider = Provider {
        peer: RandomPeerId::random(),
        service_id: provider1.into(),
    };
    let provider2 = Provider {
        peer: RandomPeerId::random(),
        service_id: provider2.into(),
    };
    client.send_particle(
        script,
        hashmap! {
            "client" => json!(client.peer_id.to_string()),
            "client2" => json!(client2.peer_id.to_string()),
            "node" => json!(client.node.to_string()),
            "provider" => json!(provider),
            "key" => json!(uuid()),
            "provider2" => json!(provider2),
            "key2" => json!(uuid()),
            "first_node" => json!(swarms[0].0.to_string()),
        },
    );

    let response = client2.receive_args();
    let providers = into_array(response[0].clone())
        .expect(format!("providers must be array, response was {:#?}", response).as_str());
    let providers: Vec<_> = providers
        .into_iter()
        .flat_map(|v| into_array(v).expect("must be array"))
        .map(|v| serde_json::from_value::<Provider>(v).expect("be provider"))
        .collect();
    let providers: HashSet<_> = providers.into_iter().collect();
    assert_eq!(providers.len(), 2);
    assert!(providers.contains(&provider));
    assert!(providers.contains(&provider2));
}
