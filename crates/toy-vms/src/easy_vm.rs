/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use async_trait::async_trait;
use std::str::FromStr;
use std::{convert::Infallible, task::Waker, time::Duration};

use avm_server::avm_runner::RawAVMOutcome;
use avm_server::SoftLimitsTriggering;
use avm_server::{AVMMemoryStats, CallResults, ParticleParameters};
use itertools::Itertools;

use aquamarine::WasmtimeWasmBackend;
use aquamarine::{AquaRuntime, ParticleEffects};
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;

pub struct EasyVM {
    delay: Option<Duration>,
}

#[async_trait]
impl AquaRuntime for EasyVM {
    type Config = Option<Duration>;
    type Error = Infallible;

    fn create_runtime(
        delay: Option<Duration>,
        _backend: WasmtimeWasmBackend,
        _: Waker,
    ) -> Result<Self, Self::Error> {
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

    async fn call(
        &mut self,
        air: impl Into<String> + Send,
        _prev_data: impl Into<Vec<u8>> + Send,
        current_data: impl Into<Vec<u8>> + Send,
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
        let soft_limits_triggering = SoftLimitsTriggering::default();
        Ok(RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data,
            call_requests: Default::default(),
            next_peer_pks: vec![next_peer],
            soft_limits_triggering,
        })
    }

    fn memory_stats(&self) -> AVMMemoryStats {
        AVMMemoryStats {
            memory_size: 0,
            total_memory_limit: None,
            allocation_rejects: None,
        }
    }
}
