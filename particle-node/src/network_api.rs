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

use aquamarine::{AquamarineApi, SendParticle, StepperEffects};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, LifecycleEvent};
use control_macro::measure;
use control_macro::unwrap_return;
use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use particle_protocol::Contact;
use particle_protocol::Particle;
use server_config::NodeConfig;

use async_std::{
    sync::Mutex,
    task::JoinHandle,
    task::{sleep, spawn},
};
use futures::{future, sink, stream::iter, task, Future, FutureExt, Sink, SinkExt, StreamExt};
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::{core::Multiaddr, swarm::NetworkBehaviour, PeerId, Swarm};
use std::time::Instant;
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

/// API provided by the network
pub struct NetworkApi {
    /// Stream of particles coming from other peers, lifted here from [[ConnectionPoolBehaviour]]
    particle_stream: BackPressuredInlet<Particle>,
    /// Number of concurrently processed particles
    particle_parallelism: usize,
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
        particle_parallelism: usize,
        kademlia: KademliaApi,
        connection_pool: ConnectionPoolApi,
        bootstrap_frequency: usize,
        particle_timeout: Duration,
    ) -> Self {
        Self {
            particle_stream,
            particle_parallelism,
            connectivity: Connectivity {
                kademlia,
                connection_pool,
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
        particle_failures_sink: impl Sink<String> + Clone + Unpin + Send + Sync + 'static,
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
        let particles = spawn(connectivity.process_particles(
            Some(particle_parallelism),
            particle_stream,
            aquamarine,
            particle_failures_sink,
            particle_timeout,
        ));

        NetworkTasks::new(particles, reconnect_bootstraps, run_bootstrap)
    }
}

#[derive(Clone, Debug)]
/// This structure is just a composition of Kademlia and ConnectionPool.
/// It exists solely for code conciseness (i.e. avoid tuples);
/// there's no architectural motivation behind
pub struct Connectivity {
    pub kademlia: KademliaApi,
    pub connection_pool: ConnectionPoolApi,
}

impl Connectivity {
    /// Perform effects that Aquamarine instructed us to
    pub async fn execute_effects(&self, effects: StepperEffects) {
        let ps = iter(effects.particles.into_iter().filter(|p| {
            if p.particle.is_expired() {
                log::info!("Particle {} is expired", p.particle.id);
                false
            } else {
                true
            }
        }));
        // take every particle, and try to send it concurrently
        ps.for_each_concurrent(None, move |p| {
            let SendParticle { target, particle } = p;
            let this = self.clone();
            async move {
                // resolve contact
                if let Some(contact) = this.resolve_contact(target, &particle.id).await {
                    // forward particle
                    this.send(contact, particle).await;
                }
            }
        })
        .await;
    }

    async fn resolve_contact(&self, target: PeerId, particle_id: &str) -> Option<Contact> {
        let contact = self.connection_pool.get_contact(target).await;
        let contact = if let Some(contact) = contact {
            println!(
                "resolved contact {} via CONNECTION POOL for particle {}",
                target, particle_id
            );
            // contact is connected directly to current node
            return Some(contact);
        } else {
            // contact isn't connected, have to discover it
            let contact = self.discover_peer(target).await;
            match contact {
                Ok(Some(contact)) => {
                    println!(
                        "resolved contact {} via KADEMLIA for particle {}",
                        target, particle_id
                    );
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
            log::info!("Failed to send particle {} to {}", id, contact);
        }
    }

    /// Discover a peer via Kademlia
    pub async fn discover_peer(&self, target: PeerId) -> Result<Option<Contact>, KademliaError> {
        // discover contact addresses through Kademlia
        let addresses = measure!(self.kademlia.discover_peer(target).await?);
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

    pub async fn process_particles(
        self,
        particle_parallelism: Option<usize>,
        particle_stream: BackPressuredInlet<Particle>,
        aquamarine: AquamarineApi,
        particle_failures_sink: impl Sink<String> + Clone + Unpin + Send + Sync + 'static,
        particle_timeout: Duration,
    ) {
        particle_stream
            .for_each_concurrent(particle_parallelism, move |particle| {
                let aquamarine = aquamarine.clone();
                let connectivity = self.clone();
                let mut particle_failures_sink = particle_failures_sink.clone();
                log::info!(target: "network", "Will execute particle {}", particle.id);

                let timeout = min(particle.time_to_live(), particle_timeout);
                if timeout.is_zero() {
                    log::info!("Particle {} expired", particle.id);
                    return async {}.boxed();
                }


                let particle_id = particle.id.clone();
                let p_id = particle_id.clone();
                let fut = async move {
                    let start = Instant::now();
                    // execute particle on Aquamarine
                    let stepper_effects = measure!(aquamarine.handle(particle).await);

                    match stepper_effects {
                        Ok(stepper_effects) => {
                            // perform effects as instructed by aquamarine
                            measure!(connectivity.execute_effects(stepper_effects).await);
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
                    log::trace!(target: "network", "Particle {} processing took {}", p_id, pretty(start.elapsed()));
                    if start.elapsed().as_millis() > 100 {
                        println!("Particle processing took {} ms", start.elapsed().as_millis());
                    }
                };

                async_std::io::timeout(timeout, fut.map(Ok)).map(move |r| {
                    if let Err(err) = r {
                        if timeout != particle_timeout {
                            log::info!("Particle {} expired", particle_id);
                            println!("Particle {} expired", particle_id);
                        } else {
                            log::warn!("Particle {} timed out after {}", particle_id, pretty(timeout));
                            println!("Particle {} timed out after {}", particle_id, pretty(timeout))
                        }
                    }
                }).boxed()
            })
            .await;

        log::error!("Particle stream has ended");
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
