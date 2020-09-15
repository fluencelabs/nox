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

use crate::config::AppServicesConfig;
use fluence_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::{
    identify::Identify,
    identity::{ed25519, PublicKey},
    ping::{Ping, PingConfig, PingEvent},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use particle_actors::Plumber;
use particle_dht::{DHTConfig, ParticleDHT};
use prometheus::Registry;
use std::collections::VecDeque;
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(ServerBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct ServerBehaviour {
    identity: Identify,
    ping: Ping,
    plumber: Plumber,
    dht: ParticleDHT,
    #[behaviour(ignore)]
    events: VecDeque<SwarmEventType>,
}

impl ServerBehaviour {
    pub fn new(
        key_pair: ed25519::Keypair,
        local_peer_id: PeerId,
        _listening_addresses: Vec<Multiaddr>,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
        _services_config: AppServicesConfig,
    ) -> Self {
        let local_public_key = PublicKey::Ed25519(key_pair.public());
        let identity = Identify::new(
            "/fluence/faas/1.0.0".into(),
            "0.1.0".into(),
            local_public_key,
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));
        let dht_config = DHTConfig {
            peer_id: local_peer_id,
            keypair: key_pair,
        };

        Self {
            identity,
            ping,
            plumber: Plumber::new(),
            dht: ParticleDHT::new(dht_config, trust_graph, registry),
            events: Default::default(),
        }
    }

    /// Dials bootstrap nodes
    pub fn dial_bootstrap_nodes(&mut self) {
        /*
                // TODO: how to avoid collect?
                let bootstrap_nodes: Vec<_> = self.bootstrapper.bootstrap_nodes.iter().cloned().collect();
                if bootstrap_nodes.is_empty() {
                    log::warn!("No bootstrap nodes found. Am I the only one? :(");
                }
                for maddr in bootstrap_nodes {
                    self.dial(maddr)
                }
        */
    }
    /*
        pub fn bootstrap(&mut self) {
            self.router.bootstrap()
        }
    */
    pub(super) fn dial(&mut self, maddr: Multiaddr) {
        self.events
            .push_back(libp2p::swarm::NetworkBehaviourAction::DialAddress { address: maddr })
    }

    event_polling!(custom_poll, events, SwarmEventType);
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ServerBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for ServerBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
