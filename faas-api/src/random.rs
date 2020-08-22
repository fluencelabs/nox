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

use crate::{relay, Address, FunctionCall, Protocol};
use fluence_libp2p::RandomPeerId;
use libp2p::identity::ed25519::Keypair;
use libp2p::identity::PublicKey::Ed25519;
use rand::Rng;
use serde_json::json;
use uuid::Uuid;

/// Used in tests, but can't be marked as `#[cfg(tests)]` because it'll not be possible to export
impl Address {
    pub fn random_relay_unsigned() -> Self {
        relay!(RandomPeerId::random(), RandomPeerId::random())
    }

    pub fn random_relay() -> Self {
        let node = RandomPeerId::random();
        let client_kp = Keypair::generate();
        let client = Ed25519(client_kp.public()).into_peer_id();

        relay!(node, client, client_kp)
    }

    pub fn random() -> Self {
        let addr: Self = Protocol::random().into();

        if rand::random() {
            return addr.append(Protocol::random());
        }

        addr
    }
}

impl Protocol {
    pub fn random() -> Self {
        let rng = rand::thread_rng();
        let rnd_str = || {
            rng.sample_iter(rand::distributions::Alphanumeric)
                .take(10)
                .collect()
        };
        let i = rand::random::<usize>() % 3;
        match i {
            0 => Protocol::Providers(rnd_str()),
            1 => Protocol::Peer(RandomPeerId::random()),
            2 => Protocol::Client(RandomPeerId::random()),
            _ => Protocol::Hashtag(rnd_str()),
        }
    }
}

impl FunctionCall {
    pub fn random() -> Self {
        let rng = rand::thread_rng();
        let rnd_str = || {
            rng.sample_iter(rand::distributions::Alphanumeric)
                .take(10)
                .collect()
        };
        let maybe_str = || {
            if rand::random() {
                Some(rnd_str())
            } else {
                None
            }
        };
        let maybe_call_parameters = || {
            if rand::random() {
                Some(fluence_app_service::CallParameters {
                    call_id: rnd_str(),
                    user_name: rnd_str(),
                })
            } else {
                None
            }
        };
        Self {
            uuid: Uuid::new_v4().to_string(),
            target: Address::random().into(),
            reply_to: Address::random().into(),
            module: maybe_str(),
            fname: maybe_str(),
            arguments: json!({ rnd_str(): rnd_str() }),
            call_parameters: maybe_call_parameters(),
            name: maybe_str(),
            sender: Address::random(),
        }
    }
}
