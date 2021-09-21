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

//! This file describes `NetworkApi` and `Connectivity`.
//!
//! These are basically the things we can call and manipulate in order to
//! change something in the network, or receive something from it.
//!
//! The most fundamental effect here is that of
//! - receiving a particle
//! - executing it through Aquamarine
//! - forwarding the particle to the next peers

use crate::network_tasks::NetworkTasks;

use aquamarine::{AquamarineApi, Observation, ParticleEffects};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, LifecycleEvent};

use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use particle_protocol::Contact;
use particle_protocol::Particle;

use async_std::task::{sleep, spawn};
use futures::{stream::iter, FutureExt, SinkExt, StreamExt};
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::{core::Multiaddr, PeerId};
use std::{cmp::min, collections::HashSet, time::Duration, time::Instant};

/// API provided by the network
pub struct NetworkApi {
    /// Stream of particles coming from other peers, lifted here from [[ConnectionPoolBehaviour]]
    particle_stream: BackPressuredInlet<Particle>,
    /// Number of concurrently processed particles
    particle_parallelism: Option<usize>,
    /// Kademlia and ConnectionPool in a single Clone-able structure
    connectivity: Connectivity,
    /// Bootstrap will be executed after [1, N, 2*N, 3*N, ...] bootstrap nodes connected
    /// This setting specify that N.
    bootstrap_frequency: usize,
    /// Timeout for all particle execution
    particle_timeout: Duration,
}

impl NetworkApi {
    pub fn new(
        particle_stream: BackPressuredInlet<Particle>,
        particle_parallelism: Option<usize>,
        kademlia: KademliaApi,
        connection_pool: ConnectionPoolApi,
        bootstrap_frequency: usize,
        particle_timeout: Duration,
        current_peer_id: PeerId,
    ) -> Self {
        Self {
            particle_stream,
            particle_parallelism,
            connectivity: Connectivity {
                kademlia,
                connection_pool,
                current_peer_id,
            },
            bootstrap_frequency,
            particle_timeout,
        }
    }

    /// Return connectivity API to access Connection Pool or Kademlia
    pub fn connectivity(&self) -> Connectivity {
        self.connectivity.clone()
    }

    /// Spawns a new task that pulls particles from `particle_stream`,
    /// then executes them on `stepper_pool`, and sends to other peers through `execute_effects`
    ///
    /// `parallelism` sets the number of simultaneously processed particles
    pub fn start(
        self,
        aquamarine: AquamarineApi,
        bootstrap_nodes: HashSet<Multiaddr>,
        particle_failures_sink: impl futures::Sink<String> + Clone + Unpin + Send + Sync + 'static,
    ) -> NetworkTasks {
        let NetworkApi {
            particle_stream,
            particle_parallelism,
            connectivity,
            bootstrap_frequency: freq,
            particle_timeout,
        } = self;
        let bs = bootstrap_nodes;
        let reconnect_bootstraps = spawn(connectivity.clone().reconnect_bootstraps(bs.clone()));
        let run_bootstrap = spawn(connectivity.clone().kademlia_bootstrap(bs, freq));

        let (obs_sink, obs_src) = futures::channel::mpsc::unbounded();

        let particle_stream = particle_stream.map(|p: Particle| Observation::from(p));
        let dispatcher = Dispatcher {
            particle_sink: obs_sink.clone(),
            aquamarine: aquamarine.clone(),
            particle_failures_sink: particle_failures_sink.clone(),
            connectivity: connectivity.clone(),
            particle_parallelism: particle_parallelism.clone(),
            particle_stream: particle_stream.clone(),
            particle_timeout: particle_timeout.clone(),
        };
        let particles = spawn(dispatcher.process_particles());

        let dispatcher = Dispatcher {
            particle_sink: obs_sink,
            particle_stream: obs_src,
            aquamarine,
            particle_failures_sink,
            connectivity,
            particle_parallelism,
            particle_timeout,
        };
        let observations = spawn(dispatcher.process_particles());

        NetworkTasks::new(particles, reconnect_bootstraps, run_bootstrap, observations)
    }
}

#[derive(Clone, Debug)]
/// This structure is just a composition of Kademlia and ConnectionPool.
/// It exists solely for code conciseness (i.e. avoid tuples);
/// there's no architectural motivation behind
pub struct Connectivity {
    pub kademlia: KademliaApi,
    pub connection_pool: ConnectionPoolApi,
    pub current_peer_id: PeerId,
}

impl Connectivity {
    async fn resolve_contact(&self, target: PeerId, particle_id: &str) -> Option<Contact> {
        let contact = self.connection_pool.get_contact(target).await;
        if let Some(contact) = contact {
            // contact is connected directly to current node
            return Some(contact);
        } else {
            // contact isn't connected, have to discover it
            let contact = self.discover_peer(target).await;
            match contact {
                Ok(Some(contact)) => {
                    // connect to the discovered contact
                    self.connection_pool.connect(contact.clone()).await;
                    return Some(contact);
                }
                Ok(None) => {
                    log::warn!("Couldn't discover {} for particle {}", target, particle_id);
                }
                Err(err) => {
                    let id = particle_id;
                    log::warn!("Failed to discover {} for particle {}: {}", target, id, err);
                }
            }
        };

        None
    }

    async fn send(&self, contact: Contact, particle: Particle) {
        log::debug!("Sending particle {} to {}", particle.id, contact);
        let id = particle.id.clone();
        let sent = self.connection_pool.send(contact.clone(), particle).await;
        if sent {
            log::info!("Sent particle {} to {}", id, contact);
        } else {
            // TODO: return & log error
            log::info!("Failed to send particle {} to {}", id, contact);
        }
    }

    /// Discover a peer via Kademlia
    pub async fn discover_peer(&self, target: PeerId) -> Result<Option<Contact>, KademliaError> {
        // discover contact addresses through Kademlia
        let addresses = self.kademlia.discover_peer(target).await?;
        if addresses.is_empty() {
            return Ok(None);
        }

        Ok(Some(Contact::new(target, addresses)))
    }

    /// Run kademlia bootstrap after first bootstrap is connected, and then every `frequency`
    pub async fn kademlia_bootstrap(self, bootstrap_nodes: HashSet<Multiaddr>, frequency: usize) {
        let kademlia = self.kademlia;
        let pool = self.connection_pool;

        // Count connected (and reconnected) bootstrap nodes
        let connections = {
            use async_std::stream::StreamExt as stream;

            let bootstrap_nodes = bootstrap_nodes.clone();
            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                if let LifecycleEvent::Connected(c) = e {
                    let mut addresses = c.addresses.iter();
                    addresses.find(|addr| bootstrap_nodes.contains(addr))?;
                    return Some(c);
                }
                None
            })
        }
        .enumerate();

        connections
            .for_each(move |(n, contact)| {
                let kademlia = kademlia.clone();
                async move {
                    if n % frequency == 0 {
                        kademlia.add_contact(contact);
                        if let Err(err) = kademlia.bootstrap().await {
                            log::warn!("Kademlia bootstrap failed: {}", err)
                        } else {
                            log::info!("Kademlia bootstrap finished");
                        }
                    }
                }
            })
            .await;
    }

    /// Dial bootstraps, and then re-dial on each disconnection
    pub async fn reconnect_bootstraps(self, bootstrap_nodes: HashSet<Multiaddr>) {
        let pool = self.connection_pool;
        let kademlia = self.kademlia;

        let disconnections = {
            use async_std::stream::StreamExt as stream;

            let bootstrap_nodes = bootstrap_nodes.clone();
            let events = pool.lifecycle_events();
            stream::filter_map(events, move |e| {
                if let LifecycleEvent::Disconnected(Contact { addresses, .. }) = e {
                    let addresses = addresses.into_iter();
                    let addresses = addresses.filter(|addr| bootstrap_nodes.contains(addr));
                    let addresses = iter(addresses.collect::<Vec<_>>());
                    return Some(addresses);
                }
                None
            })
        }
        .flatten();

        // TODO: take from config
        let max = Duration::from_secs(60);
        // TODO: exponential backoff + random?
        let delta = Duration::from_secs(5);

        let reconnect = move |kademlia: KademliaApi, pool: ConnectionPoolApi, addr: Multiaddr| async move {
            let mut delay = Duration::from_secs(0);
            loop {
                if let Some(contact) = pool.dial(addr.clone()).await {
                    log::info!("Connected bootstrap {}", contact);
                    let ok = kademlia.add_contact(contact);
                    debug_assert!(ok, "kademlia.add_contact");
                    break;
                }

                delay = min(delay + delta, max);
                log::warn!("can't connect bootstrap {} (pause {})", addr, pretty(delay));
                sleep(delay).await;
            }
        };

        let bootstraps = iter(bootstrap_nodes.clone().into_iter().collect::<Vec<_>>());
        bootstraps
            .chain(disconnections)
            .for_each_concurrent(None, |addr| reconnect(kademlia.clone(), pool.clone(), addr))
            .await;
    }
}

#[derive(Debug, Clone)]
struct Dispatcher<Src, Sink, FSink> {
    connectivity: Connectivity,
    particle_parallelism: Option<usize>,
    particle_stream: Src,
    particle_sink: Sink,
    aquamarine: AquamarineApi,
    particle_failures_sink: FSink,
    particle_timeout: Duration,
}

impl<Src, Sink, FSink> Dispatcher<Src, Sink, FSink>
where
    Src: futures::Stream<Item = Observation> + Unpin + Send + Sync + 'static,
    Sink: futures::Sink<Observation> + Clone + Unpin + Send + Sync + 'static,
    FSink: futures::Sink<String> + Clone + Unpin + Send + Sync + 'static,
{
    pub async fn process_particles(mut self) {
        self.particle_stream
            .for_each_concurrent(self.particle_parallelism, move |particle| {
                let timeout = min(particle.time_to_live(), self.particle_timeout);
                if timeout.is_zero() {
                    log::info!("Particle {} expired", particle.id);
                    return;
                }

                let particle_id = particle.id.clone();
                let fut = self.execute_particle(particle).map(Ok);

                async_std::io::timeout(timeout, fut)
                    .map(move |r| {
                        if r.is_ok() {
                            return;
                        }

                        if timeout != self.particle_timeout {
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

    async fn execute_particle(&mut self, particle: Observation) {
        let aquamarine = self.aquamarine.clone();
        let connectivity = self.connectivity.clone();
        let peer_id = connectivity.current_peer_id;
        let mut particle_failures_sink = self.particle_failures_sink.clone();
        log::info!(target: "network", "{} Will execute particle {}", peer_id, particle.id);

        let particle_id = particle.id.clone();
        let start = Instant::now();
        // execute particle on Aquamarine
        let effects = aquamarine.handle(particle).await;

        match effects {
            Ok(effects) => {
                // perform effects as instructed by aquamarine
                self.execute_effects(effects).await;
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

    /// Perform effects that Aquamarine instructed us to
    pub async fn execute_effects(&mut self, effects: ParticleEffects) {
        if effects.particle.is_expired() {
            log::info!("Particle {} is expired", effects.particle.id);
            return;
        }

        // take every particle, and try to send it concurrently
        let nps = effects.next_peers;
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
        crs.for_each_concurrent(None, move |call| {
            let particle = effects.particle.clone();
            let results = todo!("execute call requests");
            self.particle_sink
                .send(Observation::Next { particle, results })
        })
        .await;
    }
}

impl AsRef<KademliaApi> for Connectivity {
    fn as_ref(&self) -> &KademliaApi {
        &self.kademlia
    }
}

impl AsRef<ConnectionPoolApi> for Connectivity {
    fn as_ref(&self) -> &ConnectionPoolApi {
        &self.connection_pool
    }
}
