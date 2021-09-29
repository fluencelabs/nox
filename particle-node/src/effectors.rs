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

use futures::{stream::iter, FutureExt, SinkExt, StreamExt};

use aquamarine::{Observation, ParticleEffects};
use fluence_libp2p::types::{Inlet, Outlet};
use particle_closures::HostFunctions;
use avm_server::CallRequests;

use crate::connectivity::Connectivity;

#[derive(Debug, Clone)]
pub struct Effectors {
    pub connectivity: Connectivity,
    pub host_functions: HostFunctions<Connectivity>,
    pub particle_sink: Outlet<Observation>,
}

impl Effectors {
    pub fn new(
        connectivity: Connectivity,
        host_functions: HostFunctions<Connectivity>,
    ) -> (Self, Inlet<Observation>) {
        let (particle_sink, particle_source) = futures::channel::mpsc::unbounded();
        let this = Self {
            connectivity,
            host_functions,
            particle_sink,
        };

        (this, particle_source)
    }

    /// Perform effects that Aquamarine instructed us to
    pub async fn execute(&self, effects: ParticleEffects) {
        if effects.particle.is_expired() {
            log::info!("Particle {} is expired", effects.particle.id);
            return;
        }

        // take every particle, and try to send it concurrently
        let nps = iter(effects.next_peers);
        let particle = &effects.particle;
        let connectivity = self.connectivity.clone();
        nps.for_each_concurrent(None, move |target| {
            let connectivity = connectivity.clone();
            let particle = particle.clone();
            async move {
                // resolve contact
                if let Some(contact) = connectivity.resolve_contact(target, &particle.id).await {
                    // forward particle
                    connectivity.send(contact, particle).await;
                }
            }
        })
        .await;

        let crs = effects.call_requests;
        let particle = effects.particle;
        let host_functions = self.host_functions.clone();
        let particle_sink = self.particle_sink.clone();

        // let a = futures::stream::futures_unordered::FuturesUnordered:;

        async_std::task::spawn_blocking(move || {
            crs.into_iter().map(|call| )
            let results = host_functions.route()
        }).await;

        crs.for_each_concurrent(None, move |call| {
            let particle_id = particle.id.clone();
            let particle = particle.clone();
            let results = host_functions.;
            async move {
                let observation = Observation::Next { particle, results };
                let send = particle_sink.send(observation).await;
                if let Err(e) = send {
                    log::warn!("Failed to send particle {} to execution", particle_id,);
                }
            }
        })
        .await;
    }
}
