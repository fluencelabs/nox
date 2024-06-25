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

use futures::{FutureExt, StreamExt};
use prometheus_client::registry::Registry;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{instrument, Instrument};

use aquamarine::{AquamarineApi, AquamarineApiError, RemoteRoutingEffects};
use fluence_libp2p::PeerId;
use particle_protocol::{ExtendedParticle, Particle};
use peer_metrics::DispatcherMetrics;

use crate::effectors::Effectors;
use crate::tasks::Tasks;

type Effects = Result<RemoteRoutingEffects, AquamarineApiError>;

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
            .for_each_concurrent(parallelism, move |ext_particle| {
                let current_span = tracing::info_span!(parent: ext_particle.span.as_ref(), "Dispatcher::process_particles::for_each");
                let _ = current_span.enter();
                let async_span = tracing::info_span!("Dispatcher::process_particles::async");
                let aquamarine = aquamarine.clone();
                let metrics = metrics.clone();
                let particle: &Particle = ext_particle.as_ref();

                if particle.is_expired() {
                    let particle_id = &particle.id.as_str();
                    if let Some(m) = metrics {
                        m.particle_expired(particle_id);
                    }
                    tracing::info!(target: "expired", particle_id = particle_id, "Particle is expired");
                    return async {}.boxed();
                }

                async move {
                    aquamarine
                        .execute(ext_particle, None)
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
                            let async_span = tracing::info_span!(parent: effects.particle.span.as_ref(), "Dispatcher::effectors::execute");
                            // perform effects as instructed by aquamarine
                            effectors.execute(effects).instrument(async_span).await;
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
