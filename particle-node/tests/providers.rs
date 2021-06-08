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
use test_utils::{make_swarms_with_cfg, ConnectedClient, KAD_TIMEOUT};

use eyre::{eyre, ContextCompat, WrapErr};
use maplit::hashmap;
use misc::uuid;
use serde_json::json;
use std::{collections::HashSet, thread::sleep};

#[test]
fn add_providers() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let provider1 = "provider1";
    let provider2 = "provider2";
    client.send_particle(
        r#"
        (seq
            (call node ("deprecated" "add_provider") [key provider])
            (seq
                (call node ("deprecated" "add_provider") [key2 provider2])
                (seq
                    (call node ("deprecated" "get_providers") [key2] $providers)
                    (seq
                        (call node ("deprecated" "get_providers") [key] $providers)
                        (seq
                            (call node2 ("op" "identity") [])
                            (call client2 ("return" "") [$providers])
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

    let particle = client2.receive_args().wrap_err("receive").unwrap();
    let providers = particle[0]
        .as_array()
        .ok_or(eyre!("empty providers"))
        .unwrap();
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
    let mut client = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();
    let mut client2 = ConnectedClient::connect_to(swarms[0].multiaddr.clone())
        .wrap_err("connect client")
        .unwrap();

    let script = r#"
    (seq
        (call node ("kad" "neighborhood") [first_node] neighborhood)
        (seq
            (seq
                (seq
                    (fold neighborhood i
                        (seq
                            (call i ("deprecated" "add_provider") [key provider])
                            (next i)
                        )
                    )
                    (fold neighborhood i
                        (seq
                            (call i ("deprecated" "get_providers") [key] $providers)
                            (next i)
                        )
                    )
                )
                (seq
                    (fold neighborhood i
                        (seq
                            (call i ("deprecated" "add_provider") [key2 provider2])
                            (next i)
                        )
                    )
                    (fold neighborhood i
                        (seq
                            (call i ("deprecated" "get_providers") [key2] $providers)
                            (next i)
                        )
                    )
                )
            )
            (seq
                (call node ("op" "identity") [])
                (call client2 ("return" "") [$providers])
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
            "first_node" => json!(swarms[0].peer_id.to_string()),
        },
    );

    let response = client2.receive_args().wrap_err("receive").unwrap();
    let providers = into_array(response[0].clone())
        .wrap_err(format!(
            "providers must be array, response was {:#?}",
            response
        ))
        .unwrap();
    let providers: Vec<_> = providers
        .into_iter()
        .flat_map(|v| into_array(v).wrap_err("must be array").unwrap())
        .map(|v| {
            serde_json::from_value::<Provider>(v)
                .wrap_err("be provider")
                .unwrap()
        })
        .collect();
    let providers: HashSet<_> = providers.into_iter().collect();
    assert_eq!(providers.len(), 2);
    assert!(providers.contains(&provider));
    assert!(providers.contains(&provider2));
}
