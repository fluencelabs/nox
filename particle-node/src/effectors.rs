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

use avm_server::{CallRequests, CallResults};
use futures::stream::FuturesUnordered;
use futures::{stream::iter, FutureExt, SinkExt, StreamExt};

use aquamarine::{NetworkEffects, ParticleEffects};
use fluence_libp2p::types::{Inlet, Outlet};

use crate::connectivity::Connectivity;

#[derive(Clone)]
pub struct Effectors {
    pub connectivity: Connectivity,
}

impl Effectors {
    pub fn new(connectivity: Connectivity) -> Self {
        Self { connectivity }
    }

    /// Perform effects that Aquamarine instructed us to
    pub async fn execute(mut self, effects: NetworkEffects, particle_failures: Outlet<String>) {
        if effects.particle.is_expired() {
            log::info!("Particle {} is expired", effects.particle.id);
            return;
        }

        // take every next peers, and try to send particle there concurrently
        let nps = iter(effects.next_peers);
        let particle = &effects.particle;
        let connectivity = self.connectivity.clone();
        nps.for_each_concurrent(None, move |target| {
            let connectivity = connectivity.clone();
            let particle = particle.clone();
            let particle_id = particle.id.clone();
            let mut particle_failures = particle_failures.clone();
            async move {
                // resolve contact
                if let Some(contact) = connectivity.resolve_contact(target, &particle.id).await {
                    // forward particle
                    let sent = connectivity.send(contact, particle).await;
                    if sent {
                        // resolved and sent, exit
                        return;
                    }
                }
                // not exited yet, so either resolve or send failed. Report failure.
                particle_failures.send(particle_id);
            }
        })
        .await;
    }
}
