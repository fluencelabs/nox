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

use crate::config::JanusConfig;
use crate::p2p::{
    behaviour::JanusBehaviour, transport::build_transport, transport::JanusTransport,
};
use libp2p::{
    core::muxing::{StreamMuxerBox, SubstreamRef},
    floodsub::{self, Topic},
    identity, PeerId, Swarm,
};
use parity_multiaddr::{Multiaddr, Protocol};
use std::sync::Arc;
use tokio::prelude::*;

pub struct JanusService {
    pub swarm: Box<Swarm<JanusTransport, JanusBehaviour<SubstreamRef<Arc<StreamMuxerBox>>>>>,
    pub local_peer_id: PeerId,
}

impl JanusService {
    pub fn new(config: JanusConfig) -> Self {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let mut swarm = {
            let transport = build_transport(local_key.clone(), config.socket_timeout);
            let behaviour = JanusBehaviour::new(local_peer_id.clone(), local_key.public());

            Box::new(Swarm::new(transport, behaviour, local_peer_id.clone()))
        };

        let mut listen_addr = Multiaddr::from(config.listen_ip);
        listen_addr.push(Protocol::Tcp(config.listen_port));

        Swarm::listen_on(&mut swarm, listen_addr).unwrap();

        for topic in config.topics {
            let raw_topic: Topic = floodsub::TopicBuilder::new(topic).build();
            swarm.subscribe(raw_topic);
        }

        swarm.gossip_peer_state(local_peer_id.clone());

        JanusService {
            swarm,
            local_peer_id,
        }
    }

    pub fn start(mut self) {
        // TODO: maybe return future here
        let mut listening = false;
        tokio::run(futures::future::poll_fn(move || -> Result<_, ()> {
            loop {
                let poll = self.swarm.poll().expect("");
                match poll {
                    Async::Ready(Some(_)) => {}
                    Async::Ready(None) | Async::NotReady => {
                        if listening {
                            break;
                        }
                        if let Some(addr) = Swarm::listeners(&self.swarm).next() {
                            println!("P2P listening on {:?}", addr);
                            listening = true;
                        }
                    }
                }
            }
            Ok(Async::NotReady)
        }));
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        unimplemented!();
    }
}
