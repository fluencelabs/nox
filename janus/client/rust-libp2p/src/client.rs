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

use crate::behaviour::ClientServiceBehaviour;
use crate::connect_protocol::events::ToPeerEvent;
use janus_server::misc::{Inlet, Outlet};
use janus_server::peer_service::build_transport;

use async_std::task;
use futures::channel::mpsc;
use futures::future::FusedFuture;
use futures::stream::Fuse;
use futures::{select, stream::StreamExt};

use libp2p::identity::Keypair;
use libp2p::{identity, PeerId};
use log::trace;
use parity_multiaddr::Multiaddr;
use std::error::Error;
use std::time::Duration;

#[derive(Debug)]
pub struct Message {
    pub peer_id: PeerId, // either src or dst
    pub data: String,
}

impl Message {
    pub fn new(peer_id: PeerId, data: String) -> Self {
        Message { peer_id, data }
    }
}

pub struct Client {
    pub key: Keypair,
    pub peer_id: PeerId,
    /// Channel to send messages to relay node
    relay_outlet: Outlet<Message>,
    /// Stream of messages received from relay node
    client_inlet: Fuse<Inlet<Message>>,
}

impl Client {
    fn new(relay_outlet: Outlet<Message>, client_inlet: Inlet<Message>) -> Self {
        let key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(key.public());

        Client {
            key,
            peer_id,
            relay_outlet,
            client_inlet: client_inlet.fuse(),
        }
    }

    pub fn send(&self, msg: Message) {
        self.relay_outlet.unbounded_send(msg).unwrap();
    }

    pub fn receive_one(&mut self) -> impl FusedFuture<Output = Option<Message>> + '_ {
        self.client_inlet.next()
    }

    pub async fn connect(relay: Multiaddr, relay_id: PeerId) -> Result<Client, Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::unbounded();
        let (relay_outlet, relay_inlet) = mpsc::unbounded();

        let client = Client::new(relay_outlet, client_inlet);

        let mut swarm = {
            let transport = build_transport(client.key.clone(), Duration::from_secs(20));
            let behaviour = ClientServiceBehaviour::new(&client.peer_id, client.key.public());
            libp2p::Swarm::new(transport, behaviour, client.peer_id.clone())
        };

        match libp2p::Swarm::dial_addr(&mut swarm, relay.clone()) {
            Ok(_) => println!("Dialed to {:?}", relay),
            Err(e) => {
                println!("Dial to {:?} failed with {:?}", relay, e);
                Err(e)?
            }
        }

        let mut relay_inlet = relay_inlet.fuse();

        task::spawn(async move {
            loop {
                select!(
                    // Messages that were scheduled via client.send() method
                    to_relay = relay_inlet.next() => {
                        match to_relay {
                            Some(msg) => {
                                trace!("Will send to swarm to {}", msg.peer_id.to_base58());
                                // Send to relay node
                                swarm.send_message(relay_id.clone(), msg.peer_id, msg.data.into());
                            }
                            None => {}
                        }
                    }

                    // Messages that were received from relay node
                    from_swarm = swarm.select_next_some() => {
                        match from_swarm {
                            ToPeerEvent::Deliver { src_id, data } => {
                                let peer_id = PeerId::from_bytes(src_id).unwrap();
                                let data = String::from_utf8(data).unwrap();
                                trace!("Got msg {} {}", peer_id.to_base58(), data);
                                let message = Message { peer_id, data };
                                // Message will be available through client.receive_one
                                client_outlet.unbounded_send(message).unwrap();
                            }
                            ToPeerEvent::Upgrade => {}
                        }
                    }

                    // TODO: implement stop
                    // stop = client.stop
                )
            }
        });

        // TODO: return client only when connection is established, i.e. wait for an event from swarm
        Ok(client)
    }
}
