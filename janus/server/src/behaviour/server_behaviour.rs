/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use crate::Bootstrapper;
use crate::FunctionRouter;

use janus_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::{
    identify::Identify,
    identity::{ed25519, PublicKey},
    ping::{Ping, PingConfig, PingEvent},
    PeerId,
};
use parity_multiaddr::Multiaddr;
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
        trust_graph: TrustGraph,
        bootstrap_nodes: Vec<Multiaddr>,
    ) -> Self {
        let router = FunctionRouter::new(key_pair.clone(), local_peer_id, trust_graph);
        let local_public_key = PublicKey::Ed25519(key_pair.public());
        let identity = Identify::new("/janus/faas/1.0.0".into(), "0.1.0".into(), local_public_key);
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));
        let bootstrapper = Bootstrapper::new(bootstrap_nodes);

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

    pub(super) fn dial_peer(&mut self, peer_id: PeerId) {
        self.events
            .push_back(libp2p::swarm::NetworkBehaviourAction::DialPeer {
                peer_id,
                condition: libp2p::swarm::DialPeerCondition::Disconnected,
            });
    }

    event_polling!(custom_poll, events, SwarmEventType);
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for ServerBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for ServerBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
