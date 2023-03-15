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

use std::{error::Error, time::Duration};

use derivative::Derivative;
use futures::stream::StreamExt;
use libp2p::core::either::EitherError;
use libp2p::core::Multiaddr;
use libp2p::swarm::{ConnectionHandlerUpgrErr, SwarmBuilder, SwarmEvent};
use libp2p::{identity::Keypair, PeerId, Swarm};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, oneshot};
use tokio::{select, task, task::JoinHandle};

use fluence_libp2p::{build_transport, Transport};
use particle_protocol::{Particle, ProtocolConfig};

use crate::api::ParticleApi;
use crate::{behaviour::ClientBehaviour, ClientEvent};

#[derive(Debug)]
struct Command {
    node: PeerId,
    particle: Particle,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Client {
    #[derivative(Debug = "ignore")]
    pub key_pair: Keypair,
    pub peer_id: PeerId,
    /// Channel to send commands to node
    relay_outlet: mpsc::UnboundedSender<Command>,
    /// Stream of messages received from node
    client_inlet: mpsc::UnboundedReceiver<ClientEvent>,
    stop_outlet: oneshot::Sender<()>,
    pub(crate) fetched: Vec<Particle>,
}

impl Client {
    fn new(
        relay_outlet: mpsc::UnboundedSender<Command>,
        client_inlet: mpsc::UnboundedReceiver<ClientEvent>,
        stop_outlet: oneshot::Sender<()>,
        key_pair: Option<Keypair>,
    ) -> Self {
        let key = key_pair.unwrap_or_else(Keypair::generate_ed25519);
        let peer_id = key.public().to_peer_id();

        Client {
            key_pair: key,
            peer_id,
            relay_outlet,
            client_inlet,
            stop_outlet,
            fetched: vec![],
        }
    }

    pub fn send(&self, particle: Particle, node: PeerId) {
        if let Err(err) = self.relay_outlet.send(Command { node, particle }) {
            let err_msg = format!("{err:?}");
            let msg = err;
            log::warn!("Unable to send msg {:?}: {:?}", msg, err_msg)
        }
    }

    pub async fn receive_one(&mut self) -> Option<ClientEvent> {
        self.client_inlet.recv().await
    }

    pub fn stop(self) {
        if self.stop_outlet.send(()).is_err() {
            log::warn!("Unable to send stop, channel closed")
        }
    }

    pub fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        self.key_pair.sign(bytes).expect("signing error")
    }

    fn dial(
        &self,
        node: Multiaddr,
        transport: Transport,
        transport_timeout: Duration,
        protocol_config: ProtocolConfig,
    ) -> Result<Swarm<ClientBehaviour>, Box<dyn Error>> {
        let mut swarm = {
            let behaviour = ClientBehaviour::new(protocol_config);

            let transport = build_transport(transport, self.key_pair.clone(), transport_timeout);
            SwarmBuilder::with_tokio_executor(transport, behaviour, self.peer_id).build()
        };

        match Swarm::dial(&mut swarm, node.clone()) {
            Ok(_) => log::info!("{} dialed to {:?}", self.peer_id, node),
            Err(e) => {
                log::error!("Dial to {:?} failed with {:?}", node, e);
                return Err(e.into());
            }
        }

        Ok(swarm)
    }

    pub async fn connect(
        relay: Multiaddr,
        transport_timeout: Duration,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        Self::connect_with(relay, Transport::Network, None, transport_timeout).await
    }

    pub async fn connect_with(
        relay: Multiaddr,
        transport: Transport,
        key_pair: Option<Keypair>,
        transport_timeout: Duration,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::unbounded_channel();
        let (relay_outlet, mut relay_inlet) = mpsc::unbounded_channel();

        let (stop_outlet, stop_inlet) = oneshot::channel();

        let protocol_config = ProtocolConfig::new(
            transport_timeout,
            // keep alive timeout
            Duration::from_secs(10),
            transport_timeout,
        );
        let client = Client::new(relay_outlet, client_inlet, stop_outlet, key_pair);
        let mut swarm = client.dial(relay, transport, transport_timeout, protocol_config)?;
        let mut stop_inlet = Some(stop_inlet);

        let task = task::Builder::new()
            .name("Client")
            .spawn(async move {
                loop {
                    let stop_inlet = stop_inlet.as_mut().expect("Could not get stop inlet");
                    select!(
                        // Messages that were scheduled via client.send() method
                        to_relay = relay_inlet.recv() => {
                            if let Some(cmd) = to_relay {
                                Self::send_to_node(swarm.behaviour_mut(), cmd)
                            }
                        }

                        // Messages that were received from relay node
                        Some(from_relay) = swarm.next() => {
                            match Self::receive_from_node(from_relay, &client_outlet) {
                                Err(err) => {
                                    let err_msg = format!("{err:?}");
                                    let msg = err;
                                    log::warn!("unable to send {:?} to node: {:?}", msg, err_msg)
                                },
                                Ok(_v) => {},
                            }
                        },
                        _ = stop_inlet => break,
                    )
                }
            })
            .expect("Could not spawn task");

        Ok((client, task))
    }

    fn send_to_node<R: ParticleApi>(swarm: &mut R, cmd: Command) {
        let Command { node, particle } = cmd;
        swarm.send(node, particle)
    }

    #[allow(clippy::result_large_err)]
    fn receive_from_node(
        msg: SwarmEvent<
            ClientEvent,
            EitherError<ConnectionHandlerUpgrErr<std::io::Error>, libp2p::ping::Failure>,
        >,
        client_outlet: &mpsc::UnboundedSender<ClientEvent>,
    ) -> Result<(), SendError<ClientEvent>> {
        if let SwarmEvent::Behaviour(msg) = msg {
            // Message will be available through client.receive_one
            client_outlet.send(msg)
        } else {
            Ok(())
        }
    }
}
