/*
 * Copyright 2024 Fluence DAO
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
use fluence_keypair::{KeyPair, Signature};
use futures::stream::StreamExt;
use libp2p::core::Multiaddr;
use libp2p::swarm::SwarmEvent;
use libp2p::{PeerId, Swarm, SwarmBuilder};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc, oneshot};
use tokio::{select, task, task::JoinHandle};

use fluence_libp2p::{build_transport, Transport};
use particle_protocol::{Particle, ProtocolConfig};

use crate::api::ParticleApi;
use crate::behaviour::FluenceClientBehaviourEvent;
use crate::{behaviour::FluenceClientBehaviour, ClientEvent};

#[derive(Debug)]
struct Command {
    node: PeerId,
    particle: Particle,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Client {
    #[derivative(Debug = "ignore")]
    pub key_pair: KeyPair,
    pub peer_id: PeerId,
    /// Channel to send commands to node
    relay_outlet: mpsc::Sender<Command>,
    /// Stream of messages received from node
    client_inlet: mpsc::Receiver<ClientEvent>,
    stop_outlet: oneshot::Sender<()>,
    pub(crate) fetched: Vec<Particle>,
}

impl Client {
    fn new(
        relay_outlet: mpsc::Sender<Command>,
        client_inlet: mpsc::Receiver<ClientEvent>,
        stop_outlet: oneshot::Sender<()>,
        key_pair: Option<KeyPair>,
    ) -> Self {
        let key = key_pair.unwrap_or_else(KeyPair::generate_ed25519);
        let peer_id = key.get_peer_id();

        Client {
            key_pair: key,
            peer_id,
            relay_outlet,
            client_inlet,
            stop_outlet,
            fetched: vec![],
        }
    }

    pub async fn send(&self, particle: Particle, node: PeerId) {
        if let Err(err) = self.relay_outlet.send(Command { node, particle }).await {
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

    pub fn sign(&self, bytes: &[u8]) -> Signature {
        self.key_pair.sign(bytes).expect("signing error")
    }

    fn dial(
        &self,
        node: Multiaddr,
        transport: Transport,
        transport_timeout: Duration,
        idle_connection_timeout: Duration,
        protocol_config: ProtocolConfig,
        reconnect_enabled: bool,
    ) -> Result<Swarm<FluenceClientBehaviour>, Box<dyn Error>> {
        let mut swarm = {
            let public_key = self.key_pair.public();
            let behaviour =
                FluenceClientBehaviour::new(protocol_config, public_key.into(), reconnect_enabled);

            let kp = self.key_pair.clone().into();
            let transport = build_transport(transport, &kp, transport_timeout);
            SwarmBuilder::with_existing_identity(kp)
                .with_tokio()
                .with_other_transport(|_| transport)?
                .with_behaviour(|_| behaviour)?
                .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(idle_connection_timeout))
                .build()
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

    pub fn connect(
        relay: Multiaddr,
        transport_timeout: Duration,
        idle_connection_timeout: Duration,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        Self::connect_with(
            relay,
            Transport::Network,
            None,
            transport_timeout,
            idle_connection_timeout,
            true,
        )
    }

    pub fn connect_with(
        relay: Multiaddr,
        transport: Transport,
        key_pair: Option<KeyPair>,
        transport_timeout: Duration,
        idle_connection_timeout: Duration,
        reconnect_enabled: bool,
    ) -> Result<(Client, JoinHandle<()>), Box<dyn Error>> {
        let (client_outlet, client_inlet) = mpsc::channel(128);
        let (relay_outlet, mut relay_inlet) = mpsc::channel(128);

        let (stop_outlet, stop_inlet) = oneshot::channel();

        let protocol_config = ProtocolConfig::new(transport_timeout, transport_timeout);
        let client = Client::new(relay_outlet, client_inlet, stop_outlet, key_pair);
        let mut swarm = client.dial(
            relay,
            transport,
            transport_timeout,
            idle_connection_timeout,
            protocol_config,
            reconnect_enabled,
        )?;
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
                        },

                        // Messages that were received from relay node
                        Some(from_relay) = swarm.next() => {
                            match Self::receive_from_node(from_relay, &client_outlet).await {
                                Err(err) => {
                                    let err_msg = format!("{err:?}");
                                    let msg = err;
                                    log::warn!("unable to send {:?} to node: {:?}", msg, err_msg);
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
        tracing::debug!(
            particle_id = particle.id,
            "Sending particle to node {}",
            node
        );
        swarm.send(node, particle)
    }

    #[allow(clippy::result_large_err)]
    async fn receive_from_node(
        msg: SwarmEvent<FluenceClientBehaviourEvent>,
        client_outlet: &mpsc::Sender<ClientEvent>,
    ) -> Result<(), SendError<ClientEvent>> {
        if let SwarmEvent::Behaviour(FluenceClientBehaviourEvent::Client(msg)) = msg {
            // Message will be available through client.receive_one
            client_outlet.send(msg).await
        } else {
            Ok(())
        }
    }
}
