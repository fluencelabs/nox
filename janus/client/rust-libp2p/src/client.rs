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
use crate::connect_protocol::messages::ToPeerNetworkMsg;
use janus_server::misc::{Inlet, Outlet};
use janus_server::peer_service::build_transport;

use async_std::task;
use futures::channel::mpsc;
use futures::future::FusedFuture;
use futures::stream::Fuse;
use futures::{select, stream::StreamExt};

use crate::command::Command;
use crate::message::Message;
use crate::relay_api::RelayApi;

use libp2p::identity::Keypair;
use libp2p::{identity, PeerId, Swarm};
use log::trace;
use parity_multiaddr::Multiaddr;

use std::convert::TryInto;
use std::error::Error;
use std::ops::DerefMut;
use std::time::Duration;

pub struct Client {
    pub key: Keypair,
    pub peer_id: PeerId,
    /// Channel to send commands to relay node
    relay_outlet: Outlet<Command>,
    /// Stream of messages received from relay node
    client_inlet: Fuse<Inlet<Message>>,
}

impl Client {
    fn new(relay_outlet: Outlet<Command>, client_inlet: Inlet<Message>) -> Self {
        let key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(key.public());

        Client {
            key,
            peer_id,
            relay_outlet,
            client_inlet: client_inlet.fuse(),
        }
    }

    pub fn send(&self, cmd: Command) {
        self.relay_outlet.unbounded_send(cmd).unwrap();
    }

    pub fn receive_one(&mut self) -> impl FusedFuture<Output = Option<Message>> + '_ {
        self.client_inlet.next()
    }

    fn dial(&self, relay: Multiaddr) -> Result<Swarm<ClientServiceBehaviour>, Box<dyn Error>> {
        let mut swarm = {
            let transport = build_transport(self.key.clone(), Duration::from_secs(20));
            let behaviour = ClientServiceBehaviour::new(&self.peer_id, self.key.public());
            libp2p::Swarm::new(transport, behaviour, self.peer_id.clone())
        };

        match libp2p::Swarm::dial_addr(&mut swarm, relay.clone()) {
            Ok(_) => println!("{} dialed to {:?}", self.peer_id, relay),
            Err(e) => {
                println!("Dial to {:?} failed with {:?}", relay, e);
                return Err(e.into());
            }
        }

        Ok(swarm)
    }

    pub async fn connect(relay: Multiaddr, relay_id: PeerId) -> Result<Client, Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::unbounded();
        let (relay_outlet, relay_inlet) = mpsc::unbounded();

        let client = Client::new(relay_outlet, client_inlet);
        let mut swarm = client.dial(relay)?;

        let mut relay_inlet = relay_inlet.fuse();
        let client_id = client.peer_id.clone();

        task::spawn(async move {
            loop {
                select!(
                    // Messages that were scheduled via client.send() method
                    to_relay = relay_inlet.next() => {
                        match to_relay {
                            Some(cmd) => {
                                // Send to relay node
                                Self::send_to_relay(&mut swarm, relay_id.clone(), client_id.clone(), cmd)
                            }
                            None => {}
                        }
                    }

                    // Messages that were received from relay node
                    from_relay = swarm.select_next_some() => Self::receive_from_relay(from_relay, &client_outlet),

                    // TODO: implement stop
                    // stop = client.stop
                )
            }
        });

        // TODO: return client only when connection is established, i.e. wait for an event from swarm
        Ok(client)
    }

    fn send_to_relay<R: RelayApi, S: DerefMut<Target = R>>(
        swarm: &mut S,
        relay: PeerId,
        client: PeerId,
        cmd: Command,
    ) {
        match cmd {
            Command::Relay { dst, data } => {
                trace!("Will send to swarm to {}", dst.to_base58());
                swarm.relay_message(relay, dst, data.into())
            }
            Command::Provide(id) => swarm.provide(relay, id),
            Command::FindProviders(id) => swarm.find_providers(relay, client, id),
        }
    }

    fn receive_from_relay(event: ToPeerNetworkMsg, client_outlet: &Outlet<Message>) {
        let msg: Option<Message> = event
            .try_into()
            .expect("Can't parse ToPeerNetworkMsg into Message");

        // Message will be available through client.receive_one
        if let Some(msg) = msg {
            client_outlet.unbounded_send(msg).unwrap()
        }
    }
}
