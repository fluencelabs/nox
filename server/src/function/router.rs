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

use super::waiting_queues::WaitingQueues;
use crate::function::address_signature::verify_address_signatures;
use crate::function::peers::PeerStatus;
use crate::kademlia::MemoryStore;
use faas_api::{Address, FunctionCall, Protocol, ProtocolMessage};
use failure::_core::time::Duration;
use fluence_libp2p::generate_swarm_event_type;
use itertools::Itertools;
use libp2p::{
    core::either::EitherOutput,
    identity::{ed25519, ed25519::Keypair},
    kad::{Kademlia, KademliaConfig},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use std::collections::{HashMap, HashSet, VecDeque};
use trust_graph::TrustGraph;
use uuid::Uuid;

pub type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

/// Possible waiting states of the FunctionCall
#[derive(Debug)]
pub(super) enum WaitPeer {
    /// Wait for a given peer to become routable via Kademlia and forward call there
    Routable(FunctionCall),
    /// Wait for a given peer to become connected and forward call there
    Connected(FunctionCall),
    /// Get neighbourhood of a given PeerId, and send call to each peer there
    Neighborhood(FunctionCall),
}

impl WaitPeer {
    pub fn found(&self) -> bool {
        matches!(self, WaitPeer::Routable(_))
    }

    pub fn connected(&self) -> bool {
        matches!(self, WaitPeer::Connected(_))
    }

    pub fn neighborhood(&self) -> bool {
        matches!(self, WaitPeer::Neighborhood(_))
    }
}

impl Into<FunctionCall> for WaitPeer {
    fn into(self) -> FunctionCall {
        match self {
            WaitPeer::Routable(call) => call,
            WaitPeer::Connected(call) => call,
            WaitPeer::Neighborhood(call) => call,
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
/// TODO: add metrics-rs (relevant: substrate uses it, and publishes as http-endpoint for prometheus)
pub struct FunctionRouter {
    #[allow(dead_code)]
    pub(super) keypair: Keypair,
    // TODO: store peer_id as Lazy::new(|| kp.to_peer_id())?
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
    pub fn new(kp: Keypair, peer_id: PeerId, trust_graph: TrustGraph) -> Self {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5))
            .set_max_packet_size(100 * 4096 * 4096) // 100 Mb
            .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap())
            .set_connection_idle_timeout(Duration::from_secs(2_628_000_000)); // ~month
        let store = MemoryStore::new(peer_id.clone());

        Self {
            keypair: kp.clone(),
            peer_id: peer_id.clone(),
            events: VecDeque::new(),
            kademlia: Kademlia::with_config(kp, peer_id, store, cfg, trust_graph),
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

        // TODO: each hop will verify all signatures again. Optimization opportunity!
        if let Err(e) = verify_address_signatures(&target) {
            log::error!("invalid signature in target: {:?}, call: {:?}", e, call);
            self.send_error_on_call(call, format!("invalid signature in target: {:?}", e));
            return;
        }

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
                    // If targeted to local, terminate locally, don't forward to network
                    let ttl = if is_local { 0 } else { 1 };
                    // target will be like: /client/QmClient/service/QmService
                    self.pass_to_local_service(s.into(), call.with_target(target.collect()), ttl);
                    return;
                }
                s @ Service(_) => {
                    log::info!("{} not found locally. uuid {}", &s, &call.uuid);
                    self.find_service_provider(s.into(), call.with_target(target.collect()));
                    return;
                }
                Client(id) if is_local => {
                    let id = id.clone();
                    let client = target.next().unwrap();
                    // Remove signature from target
                    match target.next() {
                        Some(Signature(_)) => {
                            let target = Address::cons(client, target);
                            self.send_to(id, Routable, call.with_target(target));
                        }
                        other => {
                            let path = target.join("");
                            self.send_error_on_call(
                                call,
                                format!("expected /signature, got '{:?}' from {}", other, path),
                            );
                        }
                    };
                    return;
                }
                Client(id) => {
                    // TODO: what if id == self.local_peer_id?
                    self.search_for_client(id.clone(), call.with_target(target.collect()));
                    return;
                }
                Signature(_) => {
                    self.send_error_on_call(
                        call,
                        "Invalid target: expected /peer, /client or /service, got /signature"
                            .into(),
                    );
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
                handler: NotifyHandler::Any,
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

    pub(super) fn uuid() -> String {
        Uuid::new_v4().to_string()
    }

    pub(super) fn send_error(&mut self, address: Address, reason: String) {
        use serde_json::json;
        let arguments = json!({ "reason": reason });
        let uuid = Self::uuid();
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
