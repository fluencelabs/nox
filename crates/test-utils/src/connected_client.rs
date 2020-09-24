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

use super::misc::Result;
use crate::{make_swarms, CreatedSwarm, SHORT_TIMEOUT, TIMEOUT};

use fluence_client::{Client, ClientEvent, Transport};

use async_std::{future::timeout, task};
use core::ops::Deref;
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use particle_protocol::Particle;

#[derive(Debug)]
pub struct ConnectedClient {
    pub client: Client,
    pub node: PeerId,
    pub node_address: Multiaddr,
}

impl Deref for ConnectedClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl ConnectedClient {
    pub fn connect_to(node_address: Multiaddr) -> Result<Self> {
        use core::result::Result;
        use std::io::{Error, ErrorKind};

        let transport = Transport::from_maddr(&node_address);
        let connect = async move {
            let (mut client, _) = Client::connect_with(node_address.clone(), transport)
                .await
                .expect("sender connected");
            let result: Result<_, Error> = if let Some(ClientEvent::NewConnection {
                peer_id, ..
            }) = client.receive_one().await
            {
                Ok(ConnectedClient {
                    client,
                    node: peer_id,
                    node_address,
                })
            } else {
                Err(ErrorKind::ConnectionAborted.into())
            };

            result
        };
        Ok(task::block_on(timeout(TIMEOUT, connect))??)
    }

    pub fn new() -> Result<Self> {
        let swarm = make_swarms(3).into_iter().next().unwrap();
        let CreatedSwarm(node, addr1, _) = swarm;

        let connect = async move {
            let (mut client, _) = Client::connect_with(addr1.clone(), Transport::Memory)
                .await
                .expect("sender connected");
            client.receive_one().await;

            ConnectedClient {
                client,
                node,
                node_address: addr1,
            }
        };
        Ok(task::block_on(timeout(TIMEOUT, connect))?)
    }

    pub fn make_clients() -> Result<(Self, Self)> {
        let swarms = make_swarms(3);
        let mut swarms = swarms.into_iter();
        let CreatedSwarm(peer_id1, addr1, _) = swarms.next().expect("get swarm");
        let CreatedSwarm(peer_id2, addr2, _) = swarms.next().expect("get swarm");

        let connect = async move {
            let (mut first, _) = Client::connect_with(addr1.clone(), Transport::Memory)
                .await
                .expect("first connected");
            first.receive_one().await;

            let first = ConnectedClient {
                client: first,
                node: peer_id1,
                node_address: addr1,
            };

            let (mut second, _) = Client::connect_with(addr2.clone(), Transport::Memory)
                .await
                .expect("second connected");
            second.receive_one().await;

            let second = ConnectedClient {
                client: second,
                node: peer_id2,
                node_address: addr2,
            };

            (first, second)
        };

        Ok(task::block_on(timeout(TIMEOUT, connect))?)
    }

    pub fn send(&self, particle: Particle) {
        self.client.send(particle, self.node.clone())
    }

    pub fn receive(&mut self) -> Particle {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(TIMEOUT, receive)).expect("get particle");

        if let Some(ClientEvent::Particle { particle, .. }) = result {
            particle
        } else {
            panic!("Expected Some(Particle), got {:?}", result)
        }
    }

    pub fn maybe_receive(&mut self) -> Option<Particle> {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(SHORT_TIMEOUT, receive))
            .ok()
            .flatten();

        result.and_then(|particle| match particle {
            ClientEvent::Particle { particle, .. } => Some(particle),
            _ => None,
        })
    }
}
