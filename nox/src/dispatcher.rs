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
use particle_protocol::ExtendedParticle;
use peer_metrics::DispatcherMetrics;
use prometheus_client::registry::Registry;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{instrument, Instrument};

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
    effectors: Effectors,
    metrics: Option<DispatcherMetrics>,
}

impl Dispatcher {
    pub fn new(
        peer_id: PeerId,
        aquamarine: AquamarineApi,
        effectors: Effectors,
        particle_parallelism: Option<usize>,
        registry: Option<&mut Registry>,
    ) -> Self {
        Self {
            peer_id,
            effectors,
            aquamarine,
            particle_parallelism,
            metrics: registry.map(|r| DispatcherMetrics::new(r, particle_parallelism)),
        }
    }
}

impl Dispatcher {
    pub fn start(
        self,
        particle_stream: mpsc::Receiver<ExtendedParticle>,
        effects_stream: mpsc::Receiver<Effects>,
    ) -> Tasks {
        log::info!("starting dispatcher");
        let particle_stream = ReceiverStream::new(particle_stream);
        let effects_stream = ReceiverStream::new(effects_stream);
        let particles = tokio::task::Builder::new()
            .name("particles")
            .spawn(
                self.clone()
                    .process_particles(particle_stream)
                    .in_current_span(),
            )
            .expect("Could not spawn task");
        let effects = tokio::task::Builder::new()
            .name("effects")
            .spawn(self.process_effects(effects_stream).in_current_span())
            .expect("Could not spawn task");

        Tasks::new("Dispatcher", vec![particles, effects])
    }

    pub async fn process_particles<Src>(self, particle_stream: Src)
    where
        Src: futures::Stream<Item = ExtendedParticle> + Unpin + Send + Sync + 'static,
    {
        let parallelism = self.particle_parallelism;
        let aquamarine = self.aquamarine;
        let metrics = self.metrics;
        particle_stream
            .for_each_concurrent(parallelism, move |particle| {
                let current_span = tracing::info_span!(parent: particle.span.as_ref(), "Dispatcher: process particle");
                let async_span = tracing::info_span!(parent: particle.span.as_ref(), "Dispatcher: async Aquamarine.execute");
                let _ = current_span.enter();
                let aquamarine = aquamarine.clone();
                let metrics = metrics.clone();

                if particle.particle.is_expired() {
                    let particle_id = particle.particle.id;
                    if let Some(m) = metrics {
                        m.particle_expired(particle_id.as_str());
                    }
                    tracing::info!(target: "expired", particle_id = particle_id, "Particle is expired");
                    return async {}.boxed();
                }

                async move {
                    aquamarine
                        .execute(particle, None)
                        // do not log errors: Aquamarine will log them fine
                        .map(|_| ())
                        .await
                }
                    .instrument(async_span)
                .boxed()
            })
            .await;

        log::error!("Particle stream has ended");
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    async fn process_effects<Src>(self, effects_stream: Src)
    where
        Src: futures::Stream<Item = Effects> + Unpin + Send + Sync + 'static,
    {
        let parallelism = self.particle_parallelism;
        let effectors = self.effectors;
        effects_stream
            .for_each_concurrent(parallelism, move |effects| {
                let effectors = effectors.clone();

                async move {
                    match effects {
                        Ok(effects) => {
                            let span = tracing::info_span!(parent: effects.particle.span.as_ref(), "Dispatcher: execute effectors");
                            // perform effects as instructed by aquamarine
                            effectors.execute(effects).instrument(span).await;
                        }
                        Err(err) => {
                            // particles are sent in fire and forget fashion, so
                            // there's nothing to do here but log
                            log::warn!("Error executing particle: {}", err);
                        }
                    };
                }
            })
            .await;

        log::error!("Effects stream has ended");
    }
}
