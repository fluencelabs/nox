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

use aquamarine::{AquaRuntime, InterpretationStats, ParticleEffects};
use avm_server::{AVMOutcome, CallResults};
use fluence_libp2p::PeerId;
use particle_protocol::Particle;

use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;
use std::str::FromStr;
use std::{convert::Infallible, task::Waker, time::Duration};

pub struct EasyVM {
    delay: Option<Duration>,
}

impl AquaRuntime for EasyVM {
    type Config = Option<Duration>;
    type Error = Infallible;

    fn create_runtime(
        delay: Option<Duration>,
        _: Waker,
    ) -> BoxFuture<'static, Result<Self, Self::Error>> {
        futures::future::ok(EasyVM { delay }).boxed()
    }

    fn into_effects(
        outcome: Result<AVMOutcome, Self::Error>,
        mut p: Particle,
        interpretation_time: Duration,
    ) -> ParticleEffects {
        let outcome = outcome.unwrap();
        p.data = outcome.data;

        ParticleEffects {
            stats: InterpretationStats {
                interpretation_time,
                new_data_len: Some(p.data.len()),
            },
            particle: p,
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
        init_user_id: PeerId,
        script: String,
        data: Vec<u8>,
        _particle_id: &str,
        _call_results: CallResults,
    ) -> Result<AVMOutcome, Self::Error> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }

        // if the script starts with '!', then emulate routing logic
        // that allows to avoid using real AVM, but still
        // describe complex topologies
        let (next_peer, data) = if script.starts_with('!') {
            // data contains peer ids separated by comma
            let next_peers = String::from_utf8_lossy(&data);
            let mut next_peers = next_peers.split(',');
            // we pop the first one and keep others
            let next_peer = String::from(next_peers.next().unwrap());

            (next_peer, next_peers.join(",").into_bytes())
        } else {
            (init_user_id.to_string(), data)
        };

        println!("next peer = {}", next_peer);

        Ok(AVMOutcome {
            data,
            call_requests: Default::default(),
            next_peer_pks: vec![next_peer],
        })
    }

    fn cleanup(&mut self, _particle_id: &str) -> Result<(), Self::Error> {
        // Nothing to cleanup in EasyVM
        Ok(())
    }
}
