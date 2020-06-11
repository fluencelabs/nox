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

use crate::{behaviour::ClientBehaviour, function_call_api::FunctionCallApi, ClientEvent};
use async_std::{task, task::JoinHandle};
use faas_api::{relay, Address, FunctionCall, Protocol};
use fluence_libp2p::{
    build_memory_transport, build_transport,
    types::{Inlet, OneshotOutlet, Outlet},
};
use futures::{
    channel::{mpsc, mpsc::TrySendError, oneshot},
    future::FusedFuture,
    select,
    stream::{Fuse, StreamExt},
    FutureExt,
};
use libp2p::{identity, identity::ed25519, PeerId, Swarm};
use parity_multiaddr::Multiaddr;
use std::{error::Error, ops::DerefMut, time::Duration};

pub enum Transport {
    Memory,
    Network,
}

impl Transport {
    pub fn is_network(&self) -> bool {
        matches!(self, Transport::Network)
    }

    pub fn from_maddr(maddr: &Multiaddr) -> Self {
        use parity_multiaddr::Protocol::Memory;
        if maddr.iter().any(|p| matches!(p, Memory(_))) {
            Transport::Memory
        } else {
            Transport::Network
        }
    }
}

#[derive(Debug)]
struct Command {
    node: PeerId,
    call: FunctionCall,
}

#[derive(Debug)]
pub struct Client {
    pub key_pair: ed25519::Keypair,
    pub peer_id: PeerId,
    /// Channel to send commands to node
    relay_outlet: Outlet<Command>,
    /// Stream of messages received from node
    client_inlet: Fuse<Inlet<ClientEvent>>,
    stop_outlet: OneshotOutlet<()>,
}

impl Client {
    fn new(
        relay_outlet: Outlet<Command>,
        client_inlet: Inlet<ClientEvent>,
        stop_outlet: OneshotOutlet<()>,
    ) -> Self {
        let key = ed25519::Keypair::generate();
        let peer_id = identity::PublicKey::Ed25519(key.public()).into_peer_id();

        Client {
            key_pair: key,
            peer_id,
            relay_outlet,
            client_inlet: client_inlet.fuse(),
            stop_outlet,
        }
    }

    pub fn send(&self, call: FunctionCall, node: PeerId) {
        if let Err(err) = self.relay_outlet.unbounded_send(Command { node, call }) {
            let err_msg = format!("{:?}", err);
            let msg = err.into_inner();
            log::warn!("Unable to send msg {:?}: {:?}", msg, err_msg)
        }
    }

    pub fn receive_one(&mut self) -> impl FusedFuture<Output = Option<ClientEvent>> + '_ {
        self.client_inlet.next()
    }

    pub fn stop(self) {
        if self.stop_outlet.send(()).is_err() {
            log::warn!("Unable to send stop, channel closed")
        }
    }

    pub fn relay_address(&self, node: PeerId) -> Address {
        let addr = relay!(node, self.peer_id.clone());
        let sig = self.sign(addr.path().as_bytes());
        addr.append(Protocol::Signature(sig))
    }

    pub fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        self.key_pair.sign(bytes)
    }

    fn dial(
        &self,
        relay: Multiaddr,
        transport: Transport,
    ) -> Result<Swarm<ClientBehaviour>, Box<dyn Error>> {
        let mut swarm = {
            let key_pair = libp2p::identity::Keypair::Ed25519(self.key_pair.clone());
            let behaviour = ClientBehaviour::new(key_pair.public().into_peer_id());

            macro_rules! swarm {
                ($transport:expr) => {{
                    let transport = $transport;
                    Swarm::new(transport, behaviour, self.peer_id.clone())
                }};
            }

            match transport {
                Transport::Memory => swarm!(build_memory_transport(key_pair)),
                Transport::Network => swarm!(build_transport(key_pair, Duration::from_secs(20))),
            }
        };

        match Swarm::dial_addr(&mut swarm, relay.clone()) {
            Ok(_) => log::info!("{} dialed to {:?}", self.peer_id, relay),
            Err(e) => {
                log::error!("Dial to {:?} failed with {:?}", relay, e);
                return Err(e.into());
            }
        }

        Ok(swarm)
    }

    pub async fn connect(relay: Multiaddr) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        Self::connect_with(relay, Transport::Network).await
    }

    pub async fn connect_with(
        relay: Multiaddr,
        transport: Transport,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::unbounded();
        let (relay_outlet, relay_inlet) = mpsc::unbounded();

        let (stop_outlet, stop_inlet) = oneshot::channel();

        let client = Client::new(relay_outlet, client_inlet, stop_outlet);
        let mut swarm = client.dial(relay, transport)?;

        let mut relay_inlet = relay_inlet.fuse();
        let mut stop = stop_inlet.into_stream().fuse();

        let task = task::spawn(async move {
            loop {
                select!(
                    // Messages that were scheduled via client.send() method
                    to_relay = relay_inlet.next() => {
                        match to_relay {
                            Some(cmd) => {
                                // Send to node
                                Self::send_to_node(&mut swarm, cmd)
                            }
                            None => {}
                        }
                    }

                    // Messages that were received from relay node
                    from_relay = swarm.select_next_some() => {
                        match Self::receive_from_node(from_relay, &client_outlet) {
                            Err(err) => {
                                let err_msg = format!("{:?}", err);
                                let msg = err.into_inner();
                                log::warn!("unable to send {:?} to node: {:?}", msg, err_msg)
                            },
                            Ok(_v) => {},
                        }
                    },
                    _ = stop.next() => break,
                )
            }
        });

        // TODO: return client only when connection is established, i.e. wait for an event from swarm
        Ok((client, task))
    }

    fn send_to_node<R: FunctionCallApi, S: DerefMut<Target = R>>(swarm: &mut S, cmd: Command) {
        match cmd {
            Command { node, call } => swarm.call(node, call),
        }
    }

    fn receive_from_node(
        msg: ClientEvent,
        client_outlet: &Outlet<ClientEvent>,
    ) -> Result<(), TrySendError<ClientEvent>> {
        // Message will be available through client.receive_one
        client_outlet.unbounded_send(msg)
    }
}
