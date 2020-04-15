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

use crate::node_service::function::FunctionRouter;
use faas_api::FunctionCall;
use libp2p::ping::{Ping, PingConfig, PingEvent};
use libp2p::{
    identify::Identify,
    identity::{ed25519, PublicKey},
    PeerId,
};

mod identify;

/// Coordinates protocols, so they can cooperate
#[derive(::libp2p::NetworkBehaviour)]
#[behaviour]
pub struct P2PBehaviour {
    router: FunctionRouter,
    identity: Identify,
    ping: Ping,
}

impl P2PBehaviour {
    pub fn new(
        key_pair: ed25519::Keypair,
        local_peer_id: PeerId,
        root_weights: Vec<(ed25519::PublicKey, u32)>,
    ) -> Self {
        let router = FunctionRouter::new(key_pair.clone(), local_peer_id, root_weights);
        let local_public_key = PublicKey::Ed25519(key_pair.public());
        let identity = Identify::new("/janus/faas/1.0.0".into(), "0.1.0".into(), local_public_key);
        let ping = Ping::new(PingConfig::new().with_keep_alive(false));

        Self {
            router,
            identity,
            ping,
        }
    }

    /// Bootstraps the node. Currently, tells Kademlia to run bootstrapping lookup.
    pub fn bootstrap(&mut self) {
        // unimplemented!()
        // TODO: bootstrap kademlia? is there any point in that?
        // self.relay.bootstrap();
    }

    pub fn call(&mut self, call: FunctionCall) {
        self.router.call(call)
    }
}

impl libp2p::swarm::NetworkBehaviourEventProcess<()> for P2PBehaviour {
    fn inject_event(&mut self, _: ()) {}
}

impl libp2p::swarm::NetworkBehaviourEventProcess<PingEvent> for P2PBehaviour {
    fn inject_event(&mut self, _: PingEvent) {}
}
