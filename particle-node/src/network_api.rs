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

use aquamarine::{AquamarineApi, SendParticle, StepperEffects};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, Contact, LifecycleEvent};
use control_macro::unwrap_return;
use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use particle_protocol::Particle;
use server_config::NodeConfig;

use async_std::task::{sleep, spawn};
use async_std::{sync::Mutex, task::JoinHandle};
use futures::stream::{self, iter};
use futures::{future, Sink, SinkExt};
use futures::{sink, task, FutureExt, StreamExt};
use humantime_serde::re::humantime::format_duration as pretty;
use libp2p::core::Multiaddr;
use libp2p::{swarm::NetworkBehaviour, PeerId, Swarm};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::{sync::Arc, task::Poll};

/// API provided by the network
pub struct NetworkApi {
    /// Stream of particles coming from other peers, lifted here from [[ConnectionPoolBehaviour]]
    particle_stream: BackPressuredInlet<Particle>,
    /// Number of concurrently processed particles
    particle_parallelism: usize,
    /// Kademlia and ConnectionPool in a single Clone-able structure
    connectivity: Connectivity,
}

impl NetworkApi {
    pub fn new(
        particle_stream: BackPressuredInlet<Particle>,
        particle_parallelism: usize,
        kademlia: KademliaApi,
        connection_pool: ConnectionPoolApi,
    ) -> Self {
        Self {
            particle_stream,
            particle_parallelism,
            connectivity: Connectivity {
                kademlia,
                connection_pool,
            },
        }
    }

    /// Return connectivity API to access Connection Pool or Kademlia
    pub fn connectivity(&self) -> Connectivity {
        self.connectivity.clone()
    }

    /// Dial bootstraps, and then re-dial on each disconnection
    pub async fn reconnect_bootstraps(
        pool: ConnectionPoolApi,
        bootstrap_nodes: HashSet<Multiaddr>,
    ) {
        let bootstraps = iter(bootstrap_nodes.clone().into_iter().collect::<Vec<_>>());
        let events = pool.lifecycle_events();
        let disconnections = {
            events
                .filter_map(move |e| {
                    if let LifecycleEvent::Disconnected(Contact { addresses, .. }) = e {
                        let addresses = addresses.into_iter();
                        let addresses = addresses.filter(|addr| bootstrap_nodes.contains(addr));
                        let addresses = iter(addresses.collect::<Vec<_>>());
                        return future::ready(Some(addresses));
                    }
                    future::ready(None)
                })
                .flatten()
        };

        // TODO: take from config
        let max = Duration::from_secs(60);
        // TODO: exponential backoff + random?
        let delta = Duration::from_secs(5);

        let reconnect = move |pool: ConnectionPoolApi, addr: Multiaddr| async move {
            let mut delay = Duration::from_secs(0);
            loop {
                if let Some(contact) = pool.dial(addr.clone()).await {
                    log::info!("Connected bootstrap {}", contact);
                    break;
                }

                delay = min(delay + delta, max);
                log::info!("can't connect bootstrap {} (pause {})", addr, pretty(delay));
                sleep(delay).await;
            }
        };

        bootstraps
            .chain(disconnections)
            .for_each_concurrent(None, |addr| reconnect(pool.clone(), addr))
            .await;
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
    ) -> JoinHandle<()> {
        spawn(async move {
            let NetworkApi {
                particle_stream,
                particle_parallelism,
                connectivity,
            } = self;

            let pool = connectivity.connection_pool.clone();
            spawn(Self::reconnect_bootstraps(pool, bootstrap_nodes));

            particle_stream
                .for_each_concurrent(particle_parallelism, move |particle| {
                    let aquamarine = aquamarine.clone();
                    let connectivity = connectivity.clone();
                    let mut particle_failures_sink = particle_failures_sink.clone();
                    async move {
                        // execute particle on Aquamarine
                        let stepper_effects = aquamarine.handle(particle).await;

                        match stepper_effects {
                            Ok(stepper_effects) => {
                                // perform effects as instructed by aquamarine
                                connectivity.execute_effects(stepper_effects).await;
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
                    }
                })
                .await;
        })
    }
}

#[derive(Clone)]
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
        let ps = iter(effects.particles);
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
