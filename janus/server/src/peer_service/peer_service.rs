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

use crate::config::config::PeerServiceConfig;
use crate::misc::{Inlet, OneshotInlet, OneshotOutlet, Outlet};
use crate::peer_service::{
    behaviour::PeerServiceBehaviour,
    events::{ToNodeMsg, ToPeerMsg},
    transport::build_transport,
};

use async_std::task;
use futures::{
    channel::{mpsc, oneshot},
    select, StreamExt,
};
use futures_util::FutureExt;
use libp2p::{identity, PeerId, Swarm, TransportError};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};

use std::io;

// TODO: code of peer service is very similar to node service -
// maybe it is worth to introduce a new trait?

/// Responsibilities:
/// - Command swarm to listen for clients
/// - Handle events from nodes and send them to swarm
/// - Proxy events from swarm to node service
pub struct PeerService {
    swarm: Swarm<PeerServiceBehaviour>,
    config: PeerServiceConfig,
    inlet: Inlet<ToPeerMsg>,
}

impl PeerService {
    pub fn new(config: PeerServiceConfig) -> (Box<Self>, Outlet<ToPeerMsg>) {
        let PeerServiceConfig {
            socket_timeout,
            key_pair,
            ..
        } = config.clone();

        let local_key = match key_pair {
            Some(kp) => kp,
            None => identity::Keypair::generate_ed25519(),
        };
        let local_peer_id = PeerId::from(local_key.public());
        println!("peer service is starting with id = {}", local_peer_id);

        let swarm = {
            let transport = build_transport(local_key.clone(), socket_timeout);
            let behaviour = PeerServiceBehaviour::new(&local_peer_id, local_key.public());

            Swarm::new(transport, behaviour, local_peer_id)
        };

        let (outlet, inlet) = mpsc::unbounded();
        let peer_service = PeerService {
            swarm,
            config,
            inlet,
        };

        (Box::new(peer_service), outlet)
    }

    /// Starts peer service
    /// * `node_outlet`   – channel to send events to node service
    pub fn start(mut self: Box<Self>, node_outlet: Outlet<ToNodeMsg>) -> OneshotOutlet<()> {
        let (exit_outlet, exit_inlet) = oneshot::channel();

        self.listen().expect("Error on starting peer listener");

        task::spawn(PeerService::run_events_coordination(
            self,
            node_outlet,
            exit_inlet,
        ));

        exit_outlet
    }

    /// Starts peer service listener.
    #[inline]
    fn listen(&mut self) -> Result<(), TransportError<io::Error>> {
        let mut listen_addr = Multiaddr::from(self.config.listen_ip);
        listen_addr.push(Protocol::Tcp(self.config.listen_port));

        Swarm::listen_on(&mut self.swarm, listen_addr).map(|_| ())
    }

    /// Runs a loop which coordinates events:
    /// node service => swarm
    /// swarm        => node service
    ///
    /// Stops when a message is received on `exit_inlet`.
    #[inline]
    async fn run_events_coordination(
        self: Box<Self>,
        node_outlet: Outlet<ToNodeMsg>,
        exit_inlet: OneshotInlet<()>,
    ) {
        let mut swarm = self.swarm;
        let mut peer_inlet = self.inlet.fuse();
        let mut exit_inlet = exit_inlet.into_stream().fuse();

        loop {
            select! {
                from_node = peer_inlet.next() =>
                    match from_node {
                        Some(ToPeerMsg::Deliver {
                            src_id,
                            dst_id,
                            data,
                        }) => swarm.relay_message(src_id, dst_id, data),

                        // channel is closed when node service was shut down - break the loop
                        None => break,
                    },

                // swarm stream never ends
                from_swarm = swarm.select_next_some() => {
                    trace!("peer_service/poll: sending {:?} to node_service", from_swarm);
                    node_outlet.unbounded_send(from_swarm).unwrap();
                },

                // If any msg received on `exit`, then stop the loop
                _ = exit_inlet.next() => {
                    break;
                }
            }
        }
    }
}
