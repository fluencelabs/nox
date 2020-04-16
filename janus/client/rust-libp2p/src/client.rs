/*
 * Copyright 2019 Fluence Labs Limited
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

use janus_libp2p::{
    build_transport,
    types::{Inlet, Outlet},
};

use async_std::task;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::FusedFuture;
use futures::stream::Fuse;
use futures::{select, stream::StreamExt, FutureExt};

use crate::command::ClientCommand;

use libp2p::identity::ed25519;
use libp2p::{identity, PeerId, Swarm};
use parity_multiaddr::Multiaddr;

use crate::function_call_api::FunctionCallApi;
use async_std::task::JoinHandle;

use crate::behaviour::ClientBehaviour;
use crate::ClientEvent;
use std::error::Error;
use std::ops::DerefMut;
use std::time::Duration;

pub struct Client {
    pub key_pair: ed25519::Keypair,
    pub peer_id: PeerId,
    /// Channel to send commands to node
    relay_outlet: Outlet<ClientCommand>,
    /// Stream of messages received from node
    client_inlet: Fuse<Inlet<ClientEvent>>,
}

impl Client {
    fn new(relay_outlet: Outlet<ClientCommand>, client_inlet: Inlet<ClientEvent>) -> Self {
        let key = ed25519::Keypair::generate();
        let peer_id = identity::PublicKey::Ed25519(key.public()).into_peer_id();

        Client {
            key_pair: key,
            peer_id,
            relay_outlet,
            client_inlet: client_inlet.fuse(),
        }
    }

    pub fn send(&self, cmd: ClientCommand) {
        self.relay_outlet.unbounded_send(cmd).unwrap();
    }

    pub fn receive_one(&mut self) -> impl FusedFuture<Output = Option<ClientEvent>> + '_ {
        self.client_inlet.next()
    }

    fn dial(&self, relay: Multiaddr) -> Result<Swarm<ClientBehaviour>, Box<dyn Error>> {
        let mut swarm = {
            let key_pair = libp2p::identity::Keypair::Ed25519(self.key_pair.clone());
            let transport = build_transport(key_pair, Duration::from_secs(20));
            let behaviour = ClientBehaviour::default();
            Swarm::new(transport, behaviour, self.peer_id.clone())
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

    pub async fn connect(
        relay: Multiaddr,
        stop: oneshot::Receiver<()>,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::unbounded();
        let (relay_outlet, relay_inlet) = mpsc::unbounded();

        let client = Client::new(relay_outlet, client_inlet);
        let mut swarm = client.dial(relay)?;

        let mut relay_inlet = relay_inlet.fuse();
        let mut stop = stop.into_stream().fuse();

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
                    from_relay = swarm.select_next_some() => Self::receive_from_relay(from_relay, &client_outlet),

                    _ = stop.next() => {
                        break;
                    }
                )
            }
        });

        // TODO: return client only when connection is established, i.e. wait for an event from swarm
        Ok((client, task))
    }

    fn send_to_node<R: FunctionCallApi, S: DerefMut<Target = R>>(
        swarm: &mut S,
        cmd: ClientCommand,
    ) {
        match cmd {
            ClientCommand::Call { node, call } => swarm.call(node, call),
        }
    }

    fn receive_from_relay(msg: ClientEvent, client_outlet: &Outlet<ClientEvent>) {
        // Message will be available through client.receive_one
        client_outlet.unbounded_send(msg).unwrap()
    }
}
