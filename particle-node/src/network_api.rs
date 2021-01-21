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

use crate::node::unlocks::{unlock, unlock_f};

use async_std::sync::Mutex;
use async_std::task::JoinHandle;
use connection_pool::{ConnectionPool, ConnectionPoolApi, Contact};
use fluence_libp2p::types::BackPressuredInlet;
use futures::{task, StreamExt};
use kademlia::{KademliaApi, KademliaApiOutlet};
use libp2p::swarm::NetworkBehaviour;
use libp2p::Swarm;
use particle_actors::{SendParticle, StepperEffects, StepperPoolApi};
use particle_protocol::Particle;
use server_config::NodeConfig;
use std::sync::Arc;
use std::task::Poll;

pub struct NetworkApi {
    particle_stream: BackPressuredInlet<Particle>,
    connectivity: Connectivity,
}

impl NetworkApi {
    pub fn new(
        particle_stream: BackPressuredInlet<Particle>,
        kademlia: KademliaApiOutlet,
        connection_pool: ConnectionPoolApi,
    ) -> Self {
        Self {
            particle_stream,
            connectivity: Connectivity {
                kademlia,
                connection_pool,
            },
        }
    }

    pub fn connectivity(&self) -> Connectivity {
        self.connectivity.clone()
    }

    pub fn start(self, stepper_pool: StepperPoolApi, parallelism: usize) -> JoinHandle<()> {
        async_std::task::spawn(async move {
            let NetworkApi {
                particle_stream,
                connectivity,
            } = self;

            particle_stream
                .for_each_concurrent(parallelism, move |particle| {
                    println!("got particle! {:?}", particle);
                    let stepper_pool = stepper_pool.clone();
                    let connectivity = connectivity.clone();
                    async move {
                        let stepper_effects = stepper_pool.ingest(particle).await;

                        match stepper_effects {
                            Ok(stepper_effects) => {
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
pub struct Connectivity {
    pub(self) kademlia: KademliaApiOutlet,
    pub(self) connection_pool: ConnectionPoolApi,
}

impl Connectivity {
    pub async fn execute_effects(&self, effects: StepperEffects) {
        for SendParticle { target, particle } in effects.particles {
            let contact = self.connection_pool.get_contact(target).await;
            let contact = match contact {
                Some(contact) => contact,
                None => {
                    let (peer_id, addresses) = {
                        let r = self.kademlia.discover_peer(target).await;
                        // TODO: handle error
                        r.expect("failed to discover peer")
                    };
                    let contact = Contact {
                        peer_id,
                        // TODO: take all addresses
                        addr: addresses.into_iter().next(),
                    };
                    self.connection_pool.connect(contact.clone()).await;
                    contact
                }
            };

            self.connection_pool.send(contact, particle).await;
        }
    }
}

impl AsRef<KademliaApiOutlet> for Connectivity {
    fn as_ref(&self) -> &KademliaApiOutlet {
        &self.kademlia
    }
}

impl AsRef<ConnectionPoolApi> for Connectivity {
    fn as_ref(&self) -> &ConnectionPoolApi {
        &self.connection_pool
    }
}
