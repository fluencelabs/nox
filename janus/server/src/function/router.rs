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
use crate::function::peers::PeerStatus;
use crate::kademlia::MemoryStore;
use faas_api::{Address, FunctionCall, Protocol, ProtocolMessage};
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
use std::collections::{HashMap, HashSet, VecDeque};
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

/// Possible waiting states of the FunctionCall
#[derive(Debug)]
pub(super) enum WaitPeer {
    /// In this state, FunctionCall is waiting for peer to be found in Kademlia
    Found(FunctionCall),
    /// In this state, FunctionCall is waiting for peer to be connected
    Connected(FunctionCall),
}

impl WaitPeer {
    pub fn found(&self) -> bool {
        matches!(self, WaitPeer::Found(_))
    }

    pub fn connected(&self) -> bool {
        matches!(self, WaitPeer::Connected(_))
    }
}

impl Into<FunctionCall> for WaitPeer {
    fn into(self) -> FunctionCall {
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
/// TODO: add metrics-rs
pub struct FunctionRouter {
    pub(super) peer_id: PeerId,
    // Queue of events to send to the upper level
    pub(super) events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    pub(super) kademlia: Kademlia<MemoryStore>,
    // Services provided by this node
    // TODO: health-check local services?
    pub(super) provided_names: HashMap<Address, Address>, // ServiceId -> (forward_to: Address)
    // TODO: clear queues on timeout?
    // Calls that are waiting for address to be name-resolved
    pub(super) wait_name_resolved: WaitingQueues<Address, FunctionCall>, // Address -> FunctionCall
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
            wait_name_resolved: WaitingQueues::new(),
            wait_peer: WaitingQueues::new(),
            provided_names: HashMap::new(),
            connected_peers: HashSet::new(),
        }
    }

    // ####
    // ## Entry point
    // ###
    pub fn call(&mut self, call: FunctionCall) {
        use PeerStatus::*;

        let target = call.target.clone().filter(|t| !t.is_empty()); // TODO: how to avoid .clone() here?
        let target = match target {
            Option::Some(target) => target,
            Option::None => {
                // send error if target is empty
                log::error!("Target is not defined on call {:?}", call);
                self.send_error_on_call(call, "target is not defined or empty".to_string());
                return;
            }
        };

        let mut target = target.iter().peekable();
        let mut is_local: bool = false;
        while let Some(address) = target.peek() {
            use Protocol::*;
            match address {
                Peer(id) if self.is_local(&id) => {
                    target.next(); // Remove ourselves from target address, and continue routing
                    is_local = true;
                    continue;
                }
                Peer(id) => {
                    self.send_to(id.clone(), Unknown, call.with_target(target.collect()));
                    return;
                }
                s @ Service(_) if is_local || self.service_available_locally(s) => {
                    // target will be like: /client/QmClient/service/QmService
                    self.pass_to_local_service(s.into(), call.with_target(target.collect()));
                    return;
                }
                s @ Service(_) => {
                    log::info!("{} not found locally. uuid {}", &s, &call.uuid);
                    self.find_service_provider(s.into(), call.with_target(target.collect()));
                    return;
                }
                Client(id) if is_local => {
                    self.send_to(id.clone(), Routable, call.with_target(target.collect()));
                    return;
                }
                Client(id) => {
                    // TODO: what if id == self.local_peer_id?
                    self.search_for_client(id.clone(), call.with_target(target.collect()));
                    return;
                }
            }
        }

        log::warn!("Invalid target in call {:?}", call);
        // TODO: this error is not helpful
        self.send_error_on_call(call, "invalid target in call".into());
    }

    // ####
    // ## Sending calls
    // ###

    // Schedule sending a call to unknown peer
    pub(super) fn send_to(&mut self, to: PeerId, expected: PeerStatus, call: FunctionCall) {
        use PeerStatus::*;

        let status = self.peer_status(&to);
        // Check if peer is in expected (or better) status
        if status < expected {
            // TODO: this error is not helpful. Example of helpful error: "Peer wasn't found via GetClosestPeers"
            //       consider custom errors for different pairs of (status, expected)
            #[rustfmt::skip]
            let err_msg = format!("unexpected status. Got {:?} expected {:?}", status, expected);
            #[rustfmt::skip]
            log::error!("Can't send call {:?} to peer {}: {}", call, to, err_msg);
            self.send_error_on_call(call, err_msg);
            return;
        }

        match status {
            Connected => self.send_to_connected(to, call),
            Routable => self.connect_then_send(to, call),
            Unknown => self.search_then_send(to, call),
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
        let arguments = json!({ "reason": reason, "call": call });
        let reply_to = call.reply_to.take();

        if let Some(reply_to) = reply_to {
            let call = FunctionCall {
                target: Some(reply_to),
                arguments,
                reply_to: None, // TODO: sure?
                uuid: format!("error_{}", call.uuid),
                name: call.name,
            };
            self.call(call)
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
            target: Some(address),
            arguments,
            reply_to: None, // TODO: sure?
            uuid: format!("error_{}", uuid),
            name: None,
        };
        self.call(call)
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
            node_id,
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
