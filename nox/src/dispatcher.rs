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

use aquamarine::{AquamarineApi, AquamarineApiError, RoutingEffects};
use fluence_libp2p::PeerId;
use futures::{FutureExt, StreamExt};
use particle_protocol::Particle;
use peer_metrics::DispatcherMetrics;
use prometheus_client::registry::Registry;
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};

use crate::effectors::Effectors;
use crate::tasks::Tasks;

type Effects = Result<RoutingEffects, AquamarineApiError>;

#[derive(Clone)]
pub struct Dispatcher {
    #[allow(unused)]
    peer_id: PeerId,
    /// Number of concurrently processed particles
    particle_parallelism: Option<usize>,
    aquamarine: AquamarineApi,
    particle_failures_sink: mpsc::UnboundedSender<String>,
    effectors: Effectors,
    metrics: Option<DispatcherMetrics>,
}

impl Dispatcher {
    pub fn new(
        peer_id: PeerId,
        aquamarine: AquamarineApi,
        effectors: Effectors,
        particle_failures_sink: mpsc::UnboundedSender<String>,
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
        particle_stream: mpsc::Receiver<Particle>,
        effects_stream: mpsc::UnboundedReceiver<Effects>,
    ) -> Tasks {
        log::info!("starting dispatcher");
        let particle_stream = ReceiverStream::new(particle_stream);
        let effects_stream = UnboundedReceiverStream::new(effects_stream);
        let particles = tokio::task::Builder::new()
            .name("particles")
            .spawn(self.clone().process_particles(particle_stream))
            .expect("Could not spawn task");
        let effects = tokio::task::Builder::new()
            .name("effects")
            .spawn(self.process_effects(effects_stream))
            .expect("Could not spawn task");

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
                let aquamarine = aquamarine.clone();
                let metrics = metrics.clone();

                if particle.is_expired() {
                    if let Some(m) = metrics {
                        m.particle_expired(&particle.id);
                    }
                    tracing::info!(particle_id = particle.id, "Particle is expired");
                    return async {}.boxed();
                }

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
                let particle_failures = particle_failures.clone();

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
                            if let Some(particle_id) = err.into_particle_id() {
                                particle_failures.send(particle_id).ok();
                            }
                        }
                    };
                }
            })
            .await;

        log::error!("Effects stream has ended");
    }
}
