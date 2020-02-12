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

use crate::config::PeerServiceConfig;
use crate::peer_service::libp2p::{
    behaviour::PeerServiceBehaviour,
    notifications::{InPeerNotification, OutPeerNotification},
    transport::build_transport,
    transport::PeerServiceTransport,
};
use async_std::task;
use futures::{
    channel::{mpsc, oneshot},
    select, StreamExt,
};
use futures_util::FutureExt;
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::Arc;

pub struct PeerService {
    pub swarm:
        Box<Swarm<PeerServiceTransport, PeerServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl PeerService {
    pub fn new(config: PeerServiceConfig) -> Self {
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

pub fn start_peer_service(
    config: PeerServiceConfig,
    peer_service_in_receiver: mpsc::UnboundedReceiver<InPeerNotification>,
    peer_service_out_sender: mpsc::UnboundedSender<OutPeerNotification>,
) -> oneshot::Sender<()> {
    let peer_service = PeerService::new(config);
    let (exit_sender, exit_receiver) = oneshot::channel();

    task::spawn(async move {
        //fusing streams
        let mut peer_service_in_receiver = peer_service_in_receiver.fuse();
        let mut peer_service_swarm = peer_service.swarm.fuse();
        let mut exit_receiver = exit_receiver.into_stream().fuse();

        loop {
            select! {
                from_node = peer_service_in_receiver.next() =>
                    match from_node {
                        Some(InPeerNotification::Relay {
                            src_id,
                            dst_id,
                            data,
                        }) => peer_service_swarm.get_mut().relay_message(src_id, dst_id, data),

                        Some(InPeerNotification::NetworkState { dst_id, state }) =>
                            peer_service_swarm
                             .get_mut()
                             .send_network_state(dst_id, state),

                        // channel is closed when node service was shut down - break the loop
                        None => break,
                    },

                // swarm stream never ends
                from_swarm = peer_service_swarm.select_next_some() => {
                    trace!("peer_service/poll: sending {:?} to peer_service", from_swarm);
                    peer_service_out_sender.unbounded_send(from_swarm).unwrap();
                },

                _ = exit_receiver.next() => {
                    break;
                }
            }
        }
    });

    exit_sender
}
