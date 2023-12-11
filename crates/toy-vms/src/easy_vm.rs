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

use std::str::FromStr;
use std::{convert::Infallible, task::Waker, time::Duration};

use avm_server::avm_runner::RawAVMOutcome;
use avm_server::{AVMMemoryStats, CallResults, ParticleParameters};
use itertools::Itertools;

use aquamarine::{AquaRuntime, ParticleEffects};
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;

pub struct EasyVM {
    delay: Option<Duration>,
}

impl AquaRuntime for EasyVM {
    type Config = Option<Duration>;
    type Error = Infallible;

    fn create_runtime(delay: Option<Duration>, _: Waker) -> Result<Self, Self::Error> {
        Ok(EasyVM { delay })
    }

    fn into_effects(
        outcome: Result<RawAVMOutcome, Self::Error>,
        _particle_id: String,
    ) -> ParticleEffects {
        let outcome = outcome.unwrap();

        ParticleEffects {
            new_data: outcome.data,
            next_peers: outcome
                .next_peer_pks
                .iter()
                .map(|peer| PeerId::from_str(peer).expect("invalid peer id"))
                .collect(),
            call_requests: outcome.call_requests,
        }
    }

    fn call(
        &mut self,
        air: impl Into<String>,
        _prev_data: impl Into<Vec<u8>>,
        current_data: impl Into<Vec<u8>>,
        particle_params: ParticleParameters<'_>,
        _call_results: CallResults,
        _key_pair: &KeyPair,
    ) -> Result<RawAVMOutcome, Self::Error> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }

        // if the script starts with '!', then emulate routing logic
        // that allows to avoid using real AVM, but still
        // describe complex topologies
        let air = air.into();
        let data = current_data.into();
        let (next_peer, data) = if air.starts_with('!') {
            // data contains peer ids separated by comma
            let next_peers = String::from_utf8_lossy(&data);
            let mut next_peers = next_peers.split(',');
            // we pop the first one and keep others
            let next_peer = String::from(next_peers.next().unwrap());

            (next_peer, next_peers.join(",").into_bytes())
        } else {
            (particle_params.init_peer_id.to_string(), data)
        };

        println!("next peer = {next_peer}");

        Ok(RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data,
            call_requests: Default::default(),
            next_peer_pks: vec![next_peer],
        })
    }

    fn memory_stats(&self) -> AVMMemoryStats {
        AVMMemoryStats {
            memory_size: 0,
            max_memory_size: None,
        }
    }
}
