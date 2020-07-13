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

use crate::Bootstrapper;
use crate::FunctionRouter;

use crate::bootstrapper::BootstrapConfig;
use crate::function::RouterConfig;
use fluence_faas::RawCoreModulesConfig;
use fluence_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::{
    identify::Identify,
    identity::{ed25519, PublicKey},
    ping::{Ping, PingConfig, PingEvent},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use prometheus::Registry;
use std::collections::VecDeque;
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(ServerBehaviour);

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct ServerBehaviour {
    bootstrapper: Bootstrapper,
    pub(super) router: FunctionRouter,
    identity: Identify,
    ping: Ping,
    #[behaviour(ignore)]
    events: VecDeque<SwarmEventType>,
}

impl ServerBehaviour {
    pub fn new(
        key_pair: ed25519::Keypair,
        local_peer_id: PeerId,
        listening_addresses: Vec<Multiaddr>,
        trust_graph: TrustGraph,
        bootstrap_nodes: Vec<Multiaddr>,
        registry: Option<&Registry>,
        faas_config: RawCoreModulesConfig,
        bs_config: BootstrapConfig,
    ) -> Self {
        let config =
            RouterConfig::new(key_pair.clone(), local_peer_id.clone(), listening_addresses);
        let router = FunctionRouter::new(config, trust_graph, registry, faas_config);
        let local_public_key = PublicKey::Ed25519(key_pair.public());
        let identity = Identify::new(
            "/fluence/faas/1.0.0".into(),
            "0.1.0".into(),
            local_public_key,
        );
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));
        let bootstrapper = Bootstrapper::new(bs_config, local_peer_id, bootstrap_nodes);

        Self {
            router,
            identity,
            ping,
            bootstrapper,
            events: Default::default(),
        }
    }

    /// Dials bootstrap nodes
    pub fn dial_bootstrap_nodes(&mut self) {
        // TODO: how to avoid collect?
        let bootstrap_nodes: Vec<_> = self.bootstrapper.bootstrap_nodes.iter().cloned().collect();
        if bootstrap_nodes.is_empty() {
            log::warn!("No bootstrap nodes found. Am I the only one? :(");
        }
        for maddr in bootstrap_nodes {
            self.dial(maddr)
        }
    }

    pub fn bootstrap(&mut self) {
        self.router.bootstrap()
    }

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
