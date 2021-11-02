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

use std::cmp::min;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use async_std::task::{sleep, spawn};
use futures::{stream::iter, FutureExt, SinkExt, StreamExt, TryFutureExt};
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::Multiaddr;

use aquamarine::{AquamarineApi, AquamarineApiError, NetworkEffects};
use fluence_libp2p::types::{BackPressuredInlet, Inlet, Outlet};
use fluence_libp2p::PeerId;
use particle_protocol::Particle;

use crate::effectors::Effectors;
use crate::tasks::Tasks;
use crate::Connectivity;

// TODO: move error into NetworkEffects
type Effects = Result<NetworkEffects, AquamarineApiError>;

#[derive(Clone)]
pub struct Dispatcher {
    peer_id: PeerId,
    /// Number of concurrently processed particles
    particle_parallelism: Option<usize>,
    aquamarine: AquamarineApi,
    particle_failures_sink: Outlet<String>,
    /// Timeout for all particle execution
    particle_timeout: Duration,
    effectors: Effectors,
}

impl Dispatcher {
    pub fn new(
        peer_id: PeerId,
        aquamarine: AquamarineApi,
        effectors: Effectors,
        particle_failures_sink: Outlet<String>,
        particle_parallelism: Option<usize>,
        particle_timeout: Duration,
    ) -> Self {
        Self {
            peer_id,
            effectors,
            aquamarine,
            particle_failures_sink,
            particle_parallelism,
            particle_timeout,
        }
    }
}

impl Dispatcher {
    pub fn start(
        self,
        particle_stream: BackPressuredInlet<Particle>,
        effects_stream: Inlet<Effects>,
    ) -> Tasks {
        log::info!("starting dispatcher");
        let particles = spawn(self.clone().process_particles(particle_stream));
        let effects = spawn(self.clone().process_effects(effects_stream));

        Tasks::new("Dispatcher", vec![particles, effects])
    }

    pub async fn process_particles<Src>(self, particle_stream: Src)
    where
        Src: futures::Stream<Item = Particle> + Unpin + Send + Sync + 'static,
    {
        let particle_timeout = self.particle_timeout;
        let parallelism = self.particle_parallelism;
        let aquamarine = self.aquamarine;
        particle_stream
            .for_each_concurrent(parallelism, move |particle| {
                let mut aquamarine = aquamarine.clone();

                if particle.is_expired() {
                    log::info!("Particle {} expired", particle.id);
                    return async {}.boxed();
                }

                let particle_id = particle.id.clone();
                async move {
                    aquamarine
                        .execute(particle, None)
                        // do not log errors: Aquamarine will log them fine
                        .map(|_| ())
                        .await
                }
                .boxed()
            })
            .await;

        log::error!("Particle stream has ended");

        // TODO: restore these logs
        // log::info!(target: "network", "{} Will execute particle {}", self.peer_id, particle.id);
        // log::trace!(target: "network", "Particle {} processing took {}", particle_id, pretty(start.elapsed()));
    }

    async fn process_effects<Src>(self, effects_stream: Src)
    where
        Src: futures::Stream<Item = Effects> + Unpin + Send + Sync + 'static,
    {
        let particle_timeout = self.particle_timeout;
        let parallelism = self.particle_parallelism;
        let effectors = self.effectors;
        let particle_failures = self.particle_failures_sink;
        effects_stream
            .for_each_concurrent(parallelism, move |effects| {
                let effectors = effectors.clone();
                let mut particle_failures = particle_failures.clone();

                async move {
                    match effects {
                        Ok(effects) => {
                            // perform effects as instructed by aquamarine
                            effectors.execute(effects).await;
                        }
                        Err(err) => {
                            // particles are sent in fire and forget fashion, so
                            // there's nothing to do here but log
                            log::warn!("Error executing particle: {}", err);
                            // and send indication about particle failure to the outer world
                            let particle_id = err.into_particle_id();
                            particle_failures.send(particle_id).await.ok();
                        }
                    };
                }
            })
            .await;

        log::error!("Effects stream has ended");
    }
}
