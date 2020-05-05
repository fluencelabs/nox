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

use super::waiting_queues::WaitingQueues;
use crate::kademlia::MemoryStore;
use faas_api::{Address, FunctionCall, ProtocolMessage};
use failure::_core::time::Duration;
use janus_libp2p::generate_swarm_event_type;
use libp2p::{
    core::either::EitherOutput,
    identity::{
        ed25519,
        ed25519::{Keypair, PublicKey},
    },
    kad::{Kademlia, KademliaConfig},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Service {
    Delegated { forward_to: Address },
}

/// Possible waiting states of the FunctionCall
#[derive(Debug)]
pub(super) enum WaitPeer {
    /// In this state, FunctionCall is waiting for peer to be found in Kademlia
    Found(FunctionCall),
    /// In this state, FunctionCall is waiting for peer to be connected
    Connected(FunctionCall),
}

impl WaitPeer {
    pub fn call(self) -> FunctionCall {
        match self {
            WaitPeer::Found(call) => call,
            WaitPeer::Connected(call) => call,
        }
    }
}

/// Router for FunctionCall-s, based on Kademlia.
/// Workhorses are WaitingQueues, which internally are HashMaps of VecDeque. Those WaitingQueue-s
///   and their usage resemble state machine (but much uglier), where FunctionCall moves from
///   one state to another by being moved from one WaitingQueue to another.
/// Possible scenario is as follows:
///   1. FunctionCall goes to `wait_provider` queue while router is searching for ServiceId's provider in Kademlia.
///   2. Once provider is found, FunctionCall is likely to be moved to `wait_peer` wrapped in
///     `WaitPeer::Found`, meaning that this particular FunctionCall is waiting while
///     target peer (`FunctionCall::target`) is found in Kademlia.
///   3. Then, once target peer is found, FunctionCall likely to be reinserted, but now wrapped in
///     `WaitPeer::Connected`, meaning it is waiting for the target peer to be successfully dialed.
///   4. Finally, once target peer is routable and connected, the call is sent there.
///
/// TODO: Latency. Latency gonna be nuts.
pub struct FunctionRouter {
    pub(super) peer_id: PeerId,
    // Queue of events to send to the upper level
    pub(super) events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    pub(super) kademlia: Kademlia<MemoryStore>,
    // Services provided by this node
    // TODO: health-check local services?
    // TODO: register these services
    pub(super) local_services: HashMap<String, Service>, // ServiceId -> Service
    // TODO: clear queues on timeout?
    // Calls to services, waiting while provider of the service is found
    pub(super) wait_provider: WaitingQueues<String, FunctionCall>, // ServiceId -> FunctionCall
    // Calls that are waiting for target peer of the call to change state (see WaitPeer)
    pub(super) wait_peer: WaitingQueues<PeerId, WaitPeer>, // PeerId -> FunctionCall
    // TODO: clear connected_peers on inject_listener_closed?
    // Mediated by inject_connected & inject_disconnected
    pub(super) connected_peers: HashSet<PeerId>,
}

// TODO: move public methods to a trait
impl FunctionRouter {
    pub fn new(kp: Keypair, peer_id: PeerId, root_weights: Vec<(PublicKey, u32)>) -> Self {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5))
            .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap())
            .set_connection_idle_timeout(Duration::from_secs(2_628_000_000)); // ~month
        let store = MemoryStore::new(peer_id.clone());
        let trust = TrustGraph::new(root_weights);

        Self {
            peer_id: peer_id.clone(),
            events: VecDeque::new(),
            kademlia: Kademlia::with_config(kp, peer_id, store, cfg, trust),
            wait_provider: WaitingQueues::new(),
            wait_peer: WaitingQueues::new(),
            local_services: HashMap::new(),
            connected_peers: HashSet::new(),
        }
    }

    // ####
    // ## Entry point
    // ###
    /*
         Flow for sending calls is as follows:
            1) If target is service
                => if service is local: execute
                => if remote: find PeerId for service provider in Kademlia, and go to 2)
            2) If target is relay:
                => if we are the target relay: set target=client, and go to 3)
                => else: set target=relay, and go to 3)
            3) If target is this node:
                => execute message locally (see `fn execute_locally`)
            4) If target is peer:
                => if not in routing table: execute FindNode for PeerId, check if connected
                => if not connected: execute DialPeer, wait for inject_connected
                => once connected: send call
    */
    pub fn call(&mut self, call: FunctionCall) {
        match &call.target {
            Some(target) => self.send_to(target.clone(), call),
            None => self.send_error_on_call(call, "target is not defined".to_string()), // TODO: is this correct?
        }
    }

    // ####
    // ## Sending calls
    // ###

    // Send call to any address
    pub(super) fn send_to(&mut self, address: Address, call: FunctionCall) {
        use Address::*;
        match address {
            Peer { peer } if self.is_local(&peer) => {
                log::info!("Executing {} locally", &call.uuid);
                self.handle_local_call(call)
            }
            Peer { peer } => {
                log::info!("Sending {} to {}", &call.uuid, peer.to_base58());
                self.send_call(peer, call)
            }
            Relay { relay, client } if self.is_local(&relay) => {
                log::info!(
                    "Relaying {} to our client {}",
                    &call.uuid,
                    client.to_base58()
                );
                self.send_call(client, call)
            }
            Relay { relay, .. } => {
                log::info!("Forwarding {} to {}", &call.uuid, relay.to_base58());
                self.send_call(relay, call)
            }
            Service { service_id } => {
                log::info!("Sending {} to service {}", &call.uuid, service_id);
                self.send_to_service(service_id, call)
            }
        }
    }

    // Schedule sending a call to unknown peer
    pub(super) fn send_call(&mut self, to: PeerId, call: FunctionCall) {
        if self.is_routable(&to) {
            self.send_to_routable(to, call);
        } else {
            self.search_for_peer(to, call);
        }
    }

    pub(super) fn send_to_routable(&mut self, to: PeerId, call: FunctionCall) {
        if self.is_connected(&to) {
            self.send_to_connected(to, call)
        } else {
            self.connect(to, call)
        }
    }

    // Schedule sending call to the peer, assuming peer is connected
    pub(super) fn send_to_connected(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                event: EitherOutput::First(ProtocolMessage::FunctionCall(call)),
                handler: NotifyHandler::All,
            })
    }

    // ####
    // ## Error handling and sending
    // ###
    pub(super) fn send_error_on_call(&mut self, mut call: FunctionCall, reason: String) {
        use serde_json::json;
        let arguments = json!({ "reason": reason });
        let reply_to = call.reply_to.take();

        if let Some(reply_to) = reply_to {
            let call = FunctionCall {
                target: Some(reply_to.clone()),
                arguments,
                reply_to: None, // TODO: sure?
                uuid: format!("error_{}", call.uuid),
                name: call.name,
            };
            self.send_to(reply_to, call)
        } else {
            log::warn!("Can't send error on call {:?}: reply_to is empty", call);
        }
    }

    pub(super) fn send_error(&mut self, address: Address, reason: String) {
        use serde_json::json;
        use uuid::Uuid;
        let arguments = json!({ "reason": reason });
        let uuid = Uuid::new_v4().to_string();
        let call = FunctionCall {
            target: Some(address.clone()),
            arguments,
            reply_to: None, // TODO: sure?
            uuid: format!("error_{}", uuid),
            name: None,
        };
        self.send_to(address, call)
    }

    // ####
    // ## Kademlia
    // ###
    pub fn add_kad_node(
        &mut self,
        node_id: PeerId,
        addresses: Vec<Multiaddr>,
        public_key: ed25519::PublicKey,
    ) {
        log::trace!(
            "adding new node {} with {:?} addresses to kademlia",
            node_id.to_base58(),
            addresses,
        );
        for addr in addresses {
            self.kademlia
                .add_address(&node_id, addr.clone(), public_key.clone());
        }
    }

    pub fn bootstrap(&mut self) {
        self.kademlia.bootstrap()
    }
}
