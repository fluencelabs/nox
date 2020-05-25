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

use crate::utils::misc::{make_swarms, CreatedSwarm, Result, SHORT_TIMEOUT, TIMEOUT};
use faas_api::{relay, Address, FunctionCall, Protocol};

use async_std::future::timeout;
use async_std::task;
use fluence_client::{Client, ClientCommand, ClientEvent, Transport};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;

#[derive(Debug)]
pub(crate) struct ConnectedClient {
    pub(crate) client: Client,
    pub(crate) node: PeerId,
    pub(crate) node_address: Multiaddr,
}

impl ConnectedClient {
    pub(crate) fn connect_to(node_address: Multiaddr) -> Result<Self> {
        use core::result::Result;
        use std::io::{Error, ErrorKind};

        let connect = async move {
            let (mut client, _) = Client::connect_with(node_address.clone(), Transport::Memory)
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

    pub(crate) fn new() -> Result<Self> {
        let swarm = make_swarms(3).into_iter().next().unwrap();
        let CreatedSwarm(node, addr1) = swarm;

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

    pub(crate) fn make_clients() -> Result<(Self, Self)> {
        let swarms = make_swarms(3);
        let mut swarms = swarms.into_iter();
        let CreatedSwarm(peer_id1, addr1) = swarms.next().expect("get swarm");
        let CreatedSwarm(peer_id2, addr2) = swarms.next().expect("get swarm");

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

    pub fn client_address(&self) -> Address {
        Protocol::Client(self.client.peer_id.clone()).into()
    }

    pub fn relay_address(&self) -> Address {
        let addr = relay!(self.node.clone(), self.client.peer_id.clone());
        let sig = self.sign(addr.path().as_bytes());
        addr.append(Protocol::Signature(sig))
    }

    pub fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        self.client.key_pair.sign(bytes)
    }

    pub fn send(&self, call: FunctionCall) {
        self.client.send(ClientCommand::Call {
            node: self.node.clone(),
            call,
        })
    }

    pub fn receive(&mut self) -> FunctionCall {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(TIMEOUT, receive)).expect("get function call");

        if let Some(ClientEvent::FunctionCall { call, .. }) = result {
            call
        } else {
            panic!("Expected Some(FunctionCall), got {:?}", result)
        }
    }

    pub fn maybe_receive(&mut self) -> Option<FunctionCall> {
        let receive = self.client.receive_one();
        let result = task::block_on(timeout(SHORT_TIMEOUT, receive))
            .ok()
            .flatten();

        result.and_then(|call| match call {
            ClientEvent::FunctionCall { call, .. } => Some(call),
            _ => None,
        })
    }
}
