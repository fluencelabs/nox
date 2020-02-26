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

use crate::config::config::Libp2pPeerServiceConfig;
use crate::peer_service::libp2p::{
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
use libp2p::{identity, PeerId, Swarm};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};

pub struct PeerService {
    pub swarm: Box<Swarm<PeerServiceBehaviour>>,
}

impl PeerService {
    pub fn new(config: Libp2pPeerServiceConfig) -> Self {
        let local_key = match config.key_pair {
            Some(kp) => kp,
            None => identity::Keypair::generate_ed25519(),
        };
        let local_peer_id = PeerId::from(local_key.public());
        println!("peer service is starting with id = {}", local_peer_id);

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let behaviour = PeerServiceBehaviour::new(&local_peer_id, local_key.public());

            Box::new(Swarm::new(transport, behaviour, local_peer_id))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));
        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        Self { swarm }
    }
}

/// Binds port to establish libp2p connections, runs peer service based on libp2p
/// * `node_inlet` – channel to receive events from node service
/// * `node_outlet`   – channel to send events to node service
/// libp2p::start_peer_service(config.peer_service_config, node_inlet, node_outlet)
pub fn start_peer_service(
    config: Libp2pPeerServiceConfig,
    node_inlet: mpsc::UnboundedReceiver<ToPeerMsg>,
    node_outlet: mpsc::UnboundedSender<ToNodeMsg>,
) -> oneshot::Sender<()> {
    let peer_service = PeerService::new(config);
    let (exit_sender, exit_receiver) = oneshot::channel();

    task::spawn(async move {
        // stream of ToNodeMsgs
        let mut swarm = peer_service.swarm;

        // fusing streams
        let mut node_inlet = node_inlet.fuse();
        let mut exit_receiver = exit_receiver.into_stream().fuse();

        loop {
            select! {
                from_node = node_inlet.next() =>
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
                from_peer = swarm.select_next_some() => {
                    trace!("peer_service/poll: sending {:?} to node_service", from_peer);
                    node_outlet.unbounded_send(from_peer).unwrap();
                },

                _ = exit_receiver.next() => {
                    break;
                }
            }
        }
    });

    exit_sender
}
