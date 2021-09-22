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
use futures::{stream::iter, FutureExt, SinkExt, StreamExt};
use libp2p::Multiaddr;

use aquamarine::{AquamarineApi, Observation};
use fluence_libp2p::types::{BackPressuredInlet, Inlet, Outlet};
use fluence_libp2p::PeerId;
use particle_closures::HostClosures;
use particle_protocol::Particle;

use crate::connectivity::Connectivity;
use crate::effectors::Effectors;
use crate::tasks::DispatcherTasks;
use crate::{Connectivity, NetworkApi};

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
    ) -> Self {
        let dispatcher = Dispatcher {
            peer_id,
            effectors,
            aquamarine,
            particle_failures_sink,
            particle_parallelism,
            particle_timeout,
        };
    }
}

impl Dispatcher {
    pub fn start(
        self,
        // Stream of particles coming from other peers, lifted here from [[ConnectionPoolBehaviour]]
        particle_stream: BackPressuredInlet<Particle>,
        // Stream of particles with executed CallRequests
        observation_stream: Inlet<Observation>,
    ) -> DispatcherTasks {
        let particle_stream = particle_stream.map(|p: Particle| Observation::from(p));
        let particles = spawn(self.clone().process_particles(particle_stream));
        let observations = spawn(self.process_particles(observation_stream));

        DispatcherTasks::new(particles, observations)
    }

    pub async fn process_particles<Src>(self, particle_stream: Src)
    where
        Src: futures::Stream<Item = Observation> + Unpin + Send + Sync + 'static,
    {
        let particle_timeout = self.particle_timeout;
        let parallelism = self.particle_parallelism;
        let this = self;
        particle_stream
            .for_each_concurrent(parallelism, move |particle| {
                let this = this.clone();

                let timeout = min(particle.time_to_live(), particle_timeout);
                if timeout.is_zero() {
                    log::info!("Particle {} expired", particle.id);
                    return async {}.boxed();
                }

                let particle_id = particle.id.clone();
                let fut = this.execute_particle(particle).map(Ok);

                async_std::io::timeout(timeout, fut)
                    .map(move |r| {
                        if r.is_ok() {
                            return;
                        }

                        if timeout != particle_timeout {
                            log::info!("Particle {} expired", particle_id);
                        } else {
                            let tout = pretty(timeout);
                            log::warn!("Particle {} timed out after {}", particle_id, tout);
                        }
                    })
                    .boxed()
            })
            .await;

        log::error!("Particle stream has ended");
    }

    async fn execute_particle(mut self, particle: Observation) {
        let mut particle_failures_sink = self.particle_failures_sink.clone();
        log::info!(target: "network", "{} Will execute particle {}", self.peer_id, particle.id);

        let particle_id = particle.id.clone();
        let start = Instant::now();
        // execute particle on Aquamarine
        let effects = self.aquamarine.handle(particle).await;

        match effects {
            Ok(effects) => {
                // perform effects as instructed by aquamarine
                self.effectors.execute(effects).await;
            }
            Err(err) => {
                // particles are sent in fire and forget fashion, so
                // there's nothing to do here but log
                log::warn!("Error executing particle: {}", err);
                // sent info that particle has failed to the outer world
                let particle_id = err.into_particle_id();
                particle_failures_sink.feed(particle_id).await.ok();
            }
        };
        log::trace!(target: "network", "Particle {} processing took {}", particle_id, pretty(start.elapsed()));
    }
}
