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
use crate::peer_service::{
    behaviour::PeerServiceBehaviour,
    notifications::{InPeerNotification, OutPeerNotification},
    transport::build_transport,
    transport::PeerServiceTransport,
};
use async_std::task;
use futures::{
    channel::{mpsc, oneshot},
    stream::StreamExt,
};
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    identity, PeerId, Swarm,
};
use log::trace;
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub struct PeerService {
    pub swarm:
        Box<Swarm<PeerServiceTransport, PeerServiceBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
}

impl PeerService {
    pub fn new(config: PeerServiceConfig) -> Arc<Mutex<Self>> {
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

        Arc::new(Mutex::new(Self { swarm }))
    }
}

pub fn start_peer_service(
    config: PeerServiceConfig,
    mut peer_service_in_receiver: mpsc::UnboundedReceiver<InPeerNotification>,
    peer_service_out_sender: mpsc::UnboundedSender<OutPeerNotification>,
) -> oneshot::Sender<()> {
    let peer_service = PeerService::new(config);

    let (exit_sender, exit_receiver) = oneshot::channel();

    task::spawn(futures::future::select(
        futures::future::poll_fn(move |cx: &mut Context| -> Poll<()> {
            loop {
                match peer_service_in_receiver.poll_next_unpin(cx) {
                    Poll::Ready(Some(e)) => match e {
                        InPeerNotification::Relay {
                            src_id,
                            dst_id,
                            data,
                        } => peer_service
                            .lock()
                            .unwrap()
                            .swarm
                            .relay_message(src_id, dst_id, data),

                        InPeerNotification::NetworkState { dst_id, state } => peer_service
                            .lock()
                            .unwrap()
                            .swarm
                            .send_network_state(dst_id, state),
                    },
                    Poll::Pending => {
                        break;
                    }
                    Poll::Ready(None) => {
                        // TODO: propagate error
                        break;
                    }
                }
            }

            loop {
                match peer_service.lock().unwrap().swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(e)) => {
                        trace!("peer_service/poll: sending {:?} to peer_service", e);

                        peer_service_out_sender.unbounded_send(e).unwrap();
                    }
                    Poll::Ready(None) => unreachable!("stream never ends"),
                    Poll::Pending => break,
                }
            }

            Poll::Pending
        }),
        exit_receiver,
    ));

    exit_sender
}
