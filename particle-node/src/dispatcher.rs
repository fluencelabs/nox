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
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

use aquamarine::{AquamarineApi, AquamarineApiError, NetworkEffects};
use fluence_libp2p::types::{BackPressuredInlet, Inlet, Outlet};
use fluence_libp2p::PeerId;
use particle_protocol::Particle;
use peer_metrics::DispatcherMetrics;

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
    effectors: Effectors,
    metrics: Option<DispatcherMetrics>,
}

impl Dispatcher {
    pub fn new(
        peer_id: PeerId,
        aquamarine: AquamarineApi,
        effectors: Effectors,
        particle_failures_sink: Outlet<String>,
        particle_parallelism: Option<usize>,
        registry: Option<&mut Registry>,
    ) -> Self {
        Self {
            peer_id,
            effectors,
            aquamarine,
            particle_failures_sink,
            particle_parallelism,
            metrics: registry.map(|r| DispatcherMetrics::new(r, particle_parallelism)),
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
        let effects = spawn(self.process_effects(effects_stream));

        Tasks::new("Dispatcher", vec![particles, effects])
    }

    pub async fn process_particles<Src>(self, particle_stream: Src)
    where
        Src: futures::Stream<Item = Particle> + Unpin + Send + Sync + 'static,
    {
        let parallelism = self.particle_parallelism;
        let aquamarine = self.aquamarine;
        let metrics = self.metrics;
        particle_stream
            .for_each_concurrent(parallelism, move |particle| {
                let mut aquamarine = aquamarine.clone();
                let metrics = metrics.clone();

                if particle.is_expired() {
                    metrics.map(|m| m.expired_particles.inc());
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
    }

    async fn process_effects<Src>(self, effects_stream: Src)
    where
        Src: futures::Stream<Item = Effects> + Unpin + Send + Sync + 'static,
    {
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
                            effectors.execute(effects, particle_failures.clone()).await;
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
