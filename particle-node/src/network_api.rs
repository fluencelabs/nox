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
//! - executing it through Aquamarine (via [[StepperPoolApi]])
//! - forwarding the particle to the next peers

use crate::node::unlocks::{unlock, unlock_f};

use connection_pool::{ConnectionPoolApi, ConnectionPoolT, Contact};
use fluence_libp2p::types::BackPressuredInlet;
use kademlia::{KademliaApi, KademliaApiT};
use particle_actors::{SendParticle, StepperEffects, StepperPoolApi};
use particle_protocol::Particle;
use server_config::NodeConfig;

use async_std::{sync::Mutex, task::JoinHandle};
use futures::{task, StreamExt};
use libp2p::{swarm::NetworkBehaviour, PeerId, Swarm};
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

    pub fn connectivity(&self) -> Connectivity {
        self.connectivity.clone()
    }

    /// Spawns a new task that pulls particles from `particle_stream`,
    /// then executes them on `stepper_pool`, and sends to other peers through `execute_effects`
    ///
    /// `parallelism` sets the number of simultaneously processed particles
    pub fn start(self, stepper_pool: StepperPoolApi) -> JoinHandle<()> {
        async_std::task::spawn(async move {
            let NetworkApi {
                particle_stream,
                particle_parallelism,
                connectivity,
            } = self;

            particle_stream
                .for_each_concurrent(particle_parallelism, move |particle| {
                    let stepper_pool = stepper_pool.clone();
                    let connectivity = connectivity.clone();
                    async move {
                        // execute particle on Aquamarine
                        let stepper_effects = stepper_pool.ingest(particle).await;

                        match stepper_effects {
                            Ok(stepper_effects) => {
                                // perform effects as instructed by aquamarine
                                connectivity.execute_effects(stepper_effects).await
                            }
                            Err(err) => {
                                // maybe particle was expired
                                log::warn!("Error executing particle, aquamarine refused")
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
    /// Perform effects that Aquamarine (through [[StepperPoolApi]]) instructed us to
    pub async fn execute_effects(&self, effects: StepperEffects) {
        // take every particle, and try to send it
        for SendParticle { target, particle } in effects.particles {
            // resolve contact
            let contact = self.connection_pool.get_contact(target).await;
            let contact = match contact {
                // contact is connected directly to current node
                Some(contact) => contact,
                // contact isn't connected, have to discover it
                None => {
                    let contact = self.discover_peer(target).await;
                    // connect to the discovered contact
                    self.connection_pool.connect(contact.clone()).await;
                    contact
                }
            };

            // forward particle
            self.connection_pool.send(contact, particle).await;
        }
    }

    /// Discover a peer via Kademlia
    pub async fn discover_peer(&self, target: PeerId) -> Contact {
        let (peer_id, addresses) = {
            // discover contact addresses through Kademlia
            let r = self.kademlia.discover_peer(target).await;
            // TODO: handle error
            r.expect("failed to discover peer")
        };
        let contact = Contact {
            peer_id,
            // TODO: take all addresses
            addr: addresses.into_iter().next(),
        };

        contact
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
