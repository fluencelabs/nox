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

use crate::node::unlocks::{unlock, unlock_f};

use aquamarine::{AquamarineApi, SendParticle, StepperEffects};
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, Contact, LifecycleEvent};
use control_macro::unwrap_return;
use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{KademliaApi, KademliaApiT, KademliaError};
use particle_protocol::Particle;
use server_config::NodeConfig;

use async_std::task::{sleep, spawn};
use async_std::{sync::Mutex, task::JoinHandle};
use futures::future;
use futures::stream::{self, iter};
use futures::{sink, task, FutureExt, StreamExt};
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

    pub async fn connect_bootstraps(pool: ConnectionPoolApi, bootstrap_nodes: HashSet<Multiaddr>) {
        stream::iter(bootstrap_nodes.clone())
            .for_each_concurrent(None, |addr| {
                let cp = pool.clone();
                async move {
                    log::info!("boostrap connecting");
                    cp.dial(addr).await;
                    log::info!("boostrap connected");
                }
            })
            .await;
    }

    pub async fn reconnect_bootstraps(
        pool: ConnectionPoolApi,
        bootstrap_nodes: HashSet<Multiaddr>,
    ) {
        let events = pool.lifecycle_events();
        let disconnections = {
            let bootstrap_nodes = bootstrap_nodes.clone();
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

        let zero = Duration::default();
        let delays: HashMap<_, _> = bootstrap_nodes
            .into_iter()
            .map(|addr| (addr, zero))
            .collect();

        disconnections
            .fold(delays, |mut map, addr| {
                let delay = unwrap_return!(map.get_mut(&addr), future::ready(map));
                let seconds = delay.as_secs();
                let pool = pool.clone();
                spawn(async move {
                    log::info!("bootstrap reconnecting, delay {}", seconds);
                    if seconds > 0 {
                        sleep(Duration::from_secs(seconds)).await;
                    }
                    if pool.dial(addr).await.is_none() {
                        log::info!("can't connect to bootstrap");
                    } else {
                        log::info!("bootstrap reconnected");
                    }
                });
                // TODO: config max delay
                // TODO: exponential?
                *delay = Duration::from_secs(min(seconds + 5, 60));
                future::ready(map)
            })
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
    ) -> JoinHandle<()> {
        spawn(async move {
            let NetworkApi {
                particle_stream,
                particle_parallelism,
                connectivity,
            } = self;

            let pool = connectivity.connection_pool.clone();
            spawn(Self::connect_bootstraps(
                pool.clone(),
                bootstrap_nodes.clone(),
            ));
            spawn(Self::reconnect_bootstraps(pool, bootstrap_nodes));

            particle_stream
                .for_each_concurrent(particle_parallelism, move |particle| {
                    let aquamarine = aquamarine.clone();
                    let connectivity = connectivity.clone();
                    async move {
                        // execute particle on Aquamarine
                        let stepper_effects = aquamarine.handle(particle).await;

                        match stepper_effects {
                            Ok(stepper_effects) => {
                                // perform effects as instructed by aquamarine
                                connectivity.execute_effects(stepper_effects).await
                            }
                            Err(err) => {
                                // particles are sent in fire and forget fashion, so
                                // there's nothing to do here but log
                                log::warn!("Error executing particle: {}", err)
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
        // take every particle, and try to send it
        for SendParticle { target, particle } in effects.particles {
            // resolve contact
            let contact = self.connection_pool.get_contact(target).await;
            let contact = if let Some(contact) = contact {
                // contact is connected directly to current node
                contact
            } else {
                // contact isn't connected, have to discover it
                let contact = self.discover_peer(target).await;
                match contact {
                    Ok(Some(contact)) => {
                        // connect to the discovered contact
                        self.connection_pool.connect(contact.clone()).await;
                        contact
                    }
                    Ok(None) => {
                        log::warn!("Couldn't discover {} for particle {}", target, particle.id);
                        continue;
                    }
                    Err(err) => {
                        let id = particle.id;
                        log::warn!("Failed to discover {} for particle {}: {}", target, id, err);
                        continue;
                    }
                }
            };

            // forward particle
            self.connection_pool.send(contact, particle).await;
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
