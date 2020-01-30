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
use log::{error, trace};
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::Arc;

pub struct PeerService {
    pub swarm:
        Box<Swarm<PeerServiceTransport, PeerServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl PeerService {
    pub fn new(config: PeerServiceConfig) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
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
    mut peer_service_in_receiver: mpsc::UnboundedReceiver<InPeerNotification>,
    peer_service_out_sender: mpsc::UnboundedSender<OutPeerNotification>,
) -> oneshot::Sender<()> {
    let mut peer_service = PeerService::new(config);

    let (exit_sender, exit_receiver) = oneshot::channel();
    let mut exit_receiver = exit_receiver.into_stream();

    task::spawn(async move {
        loop {
            select! {
                from_node = peer_service_in_receiver.next().fuse() =>
                    match from_node {
                        Some(InPeerNotification::Relay {
                            src_id,
                            dst_id,
                            data,
                        }) => peer_service.swarm.relay_message(src_id, dst_id, data),

                        Some(InPeerNotification::NetworkState { dst_id, state }) => peer_service
                            .swarm
                            .send_network_state(dst_id, state),

                        None => {
                            error!("peer_service/select: peer_service_in_receiver has unexpectedly closed");

                            // channel is closed - break the loop
                            break;
                        }
                    },

                from_swarm = peer_service.swarm.next().fuse() =>
                    match from_swarm {
                        Some(event) => {
                            trace!("peer_service/poll: sending {:?} to peer_service", event);

                            peer_service_out_sender.unbounded_send(event).unwrap();
                        },

                        None => {
                            error!("peer_service/select: swarm stream has unexpectedly ended");

                            // swarm is ended - break the loop
                            break;
                        }
                    },

                _ = exit_receiver.next().fuse() => {
                    break;
                }
            }
        }
    });

    exit_sender
}
