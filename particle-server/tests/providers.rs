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
use test_utils::{make_swarms_with_cfg, uuid, ConnectedClient, KAD_TIMEOUT};

use serde_json::json;
use std::thread::sleep;

#[test]
fn add_providers() {
    let swarms = make_swarms_with_cfg(3, |cfg| cfg);
    sleep(KAD_TIMEOUT);
    let mut client = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");
    let mut client2 = ConnectedClient::connect_to(swarms[0].1.clone()).expect("connect client");

    let provider1 = uuid();
    let provider2 = uuid();
    client.send_particle(
        format!(
            r#"
                (seq (
                    (call (%current_peer_id% ("add_provider" "") (key provider) void))
                    (seq (
                        (call (%current_peer_id% ("add_provider" "") (key2 provider2) unit))
                        (seq (
                            (call (%current_peer_id% ("get_providers" "") (key) providers[]))
                            (seq (
                                (call (%current_peer_id% ("get_providers" "") (key2) providers[]))
                                (call ("{}" ("" "") () none))
                            ))
                        ))
                    ))
                ))
        "#,
            client2.peer_id
        ),
        json!({
            "provider": {"peer": RandomPeerId::random().to_string(), "service_id": provider1},
            "key": "folex",
            "provider2": {"peer": RandomPeerId::random().to_string(), "service_id": provider2},
            "key2": "folex2",
        }),
    );

    let particle = client2.receive();
    let providers = particle.data["providers"]
        .as_array()
        .expect("non empty providers");
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
