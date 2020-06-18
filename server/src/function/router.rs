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

use super::address_signature::verify_address_signatures;
use super::config::RouterConfig;
use super::peers::PeerStatus;
use super::wait_peer::WaitPeer;
use super::waiting_queues::WaitingQueues;
use crate::kademlia::MemoryStore;
use faas_api::{hashtag, Address, FunctionCall, Protocol, ProtocolMessage};
use failure::_core::time::Duration;
use fluence_libp2p::generate_swarm_event_type;
use itertools::Itertools;
use libp2p::{
    core::either::EitherOutput,
    identity::ed25519,
    kad::{Kademlia, KademliaConfig},
    swarm::{NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use prometheus::Registry;
use std::collections::{HashMap, HashSet, VecDeque};
use trust_graph::TrustGraph;
use uuid::Uuid;

pub(crate) type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

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
    /// Router configuration info: peer id, keypair, listening addresses
    pub(super) config: RouterConfig,
    /// Queue of events to send to the upper level
    pub(super) events: VecDeque<SwarmEventType>,
    /// Underlying Kademlia node
    pub(super) kademlia: Kademlia<MemoryStore>,
    // TODO: health-check local services?
    /// Services provided by this node
    pub(super) provided_names: HashMap<Address, Address>,
    // TODO: clear queues on timeout?
    /// Calls that are waiting for address to be name-resolved
    pub(super) wait_name_resolved: WaitingQueues<Address, FunctionCall>,
    /// Calls that are waiting for target peer of the call to change state (see WaitPeer)
    pub(super) wait_peer: WaitingQueues<PeerId, WaitPeer>,
    // TODO: clear connected_peers on inject_listener_closed?
    /// Mediated by inject_connected & inject_disconnected
    pub(super) connected_peers: HashSet<PeerId>,
}

// TODO: move public methods to a trait
impl FunctionRouter {
    pub(crate) fn new(
        config: RouterConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> Self {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5))
            .set_max_packet_size(100 * 4096 * 4096) // 100 Mb
            .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap())
            .set_connection_idle_timeout(Duration::from_secs(2_628_000_000)); // ~month
        let store = MemoryStore::new(config.peer_id.clone());
        let mut kademlia = Kademlia::with_config(
            config.keypair.clone(),
            config.peer_id.clone(),
            store,
            cfg,
            trust_graph,
        );
        if let Some(registry) = registry {
            kademlia.enable_metrics(registry);
        }

        Self {
            config,
            kademlia,
            events: <_>::default(),
            wait_name_resolved: <_>::default(),
            wait_peer: <_>::default(),
            provided_names: <_>::default(),
            connected_peers: <_>::default(),
        }
    }

    /// Attempt to send given `call`.
    /// Verifies all signatures in `call.target`, if there are any
    pub fn call(&mut self, call: FunctionCall) {
        use PeerStatus::*;
        use Protocol::*;

        let target = call.target.clone().filter(|t| !t.is_empty()); // TODO: how to avoid .clone() here?
        let target = match target {
            Some(target) => target,
            None => {
                // send error if target is empty
                log::warn!("Target is not defined on call {:?}", call);
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

        loop {
            let address = match target.peek() {
                Some(address) => address,
                // No more path nodes to route, `is_local` means we're the target => pass to local services
                None if is_local && call.module.is_some() => {
                    // If targeted to local, terminate locally, don't forward to network
                    let ttl = if is_local { 0 } else { 1 };
                    // `expect` is ok here, because condition above checks module is defined
                    let module = call.module.clone().expect("module defined here");
                    // target will be like: /client/QmClient/service/QmService
                    self.pass_to_local_service(&module, call.with_target(target.collect()), ttl);
                    return;
                }
                // No more path nodes to route, target unknown => send error
                None if !is_local => {
                    log::warn!("Invalid target in call {:?}", call);
                    // TODO: this error is not helpful
                    self.send_error_on_call(
                        call,
                        "invalid target in call: ran out of address parts".into(),
                    );
                    return;
                }
                // call.module was empty, send error
                None => {
                    log::warn!("module is not defined or empty on call {:?}", call);
                    self.send_error_on_call(call, "module is not defined or empty".into());
                    return;
                }
            };
            match address {
                Peer(id) if self.is_local(&id) => {
                    target.next(); // Remove ourselves from target address, and continue routing
                    is_local = true;
                    continue;
                }
                Peer(id) => {
                    let ctx = "send message to remote peer";
                    self.send_to(id.clone(), Unknown, call.with_target(target.collect()), ctx);
                    return;
                }
                Providers(key) => {
                    // Here network transition `/providers/$key` become local identifier `#key`
                    let key = hashtag!(key);
                    // Drop /providers/$key from target, so remainder is safe to pass
                    target.next();
                    log::info!("searching for providers of {}. uuid {}", key, &call.uuid);
                    self.find_providers(key, call.with_target(target.collect()));
                    return;
                }
                Client(id) if is_local => {
                    let client_id = id.clone();
                    let client_protocol = target.next().unwrap();
                    // Remove signature from target
                    match target.next() {
                        Some(Signature(_)) => {
                            let target = Address::cons(client_protocol, target);
                            let ctx = "send message to local client";
                            self.send_to(client_id, Connected, call.with_target(target), ctx);
                        }
                        Some(other) => {
                            let path = target.join("");
                            self.send_error_on_call(
                                call,
                                format!("expected /signature, got '{:?}' from {}", other, path),
                            );
                        }
                        None => self.send_error_on_call(call, "missing relay signature".into()),
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
                Hashtag(_) => {
                    // Ignore hashtag because there's nothing else we can do about it
                    continue;
                }
            }
        }
    }

    /// Schedule sending a call to unknown peer
    pub(super) fn send_to(
        &mut self,
        to: PeerId,
        expected: PeerStatus,
        call: FunctionCall,
        ctx: &str,
    ) {
        use PeerStatus::*;

        let status = self.peer_status(&to);
        // Check if peer is in expected (or better) status
        if status < expected {
            // TODO: This error is not helpful. Example of helpful error: "Peer wasn't found via GetClosestPeers".
            //       Consider custom errors for different pairs of (status, expected)
            #[rustfmt::skip]
            let err_msg = format!("Unexpected status for {}. Got {:?} expected {:?} ({})", to, status, expected, ctx);
            #[rustfmt::skip]
            log::error!("Can't send call {:?}: {}", call, err_msg);
            self.send_error_on_call(call, err_msg);
            return;
        }

        match status {
            Connected => self.send_to_connected(to, call),
            Routable | CheckedRoutable => self.connect_then_send(to, call),
            Unknown => self.search_then_send(to, call),
        }
    }

    /// Schedule sending call to the peer, assuming peer is connected
    pub(super) fn send_to_connected(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.events
            .push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                event: EitherOutput::First(ProtocolMessage::FunctionCall(call)),
                handler: NotifyHandler::Any,
            })
    }

    /// Send call with uuid `error_$uuid` to `call.reply_to`, if it's defined
    /// Keep `call.name`, put `reason` and `call` in arguments
    pub(super) fn send_error_on_call(&mut self, call: FunctionCall, reason: String) {
        use serde_json::json;
        let arguments = json!({ "reason": reason, "call": call });

        let reply_to = call.reply_to.as_ref().unwrap_or(&call.sender);

        if reply_to != &self.config.local_address() {
            let call = FunctionCall {
                uuid: format!("error_{}", call.uuid),
                target: Some(reply_to.clone()),
                reply_to: None, // TODO: sure?
                module: None,
                fname: None,
                arguments,
                name: call.name,
                sender: self.config.local_address(),
            };
            self.call(call)
        } else {
            log::warn!("Can't send error on call {:?}: loop detected", call);
        }
    }

    /// Generate uuid v4
    pub(super) fn uuid() -> String {
        Uuid::new_v4().to_string()
    }

    /// Send error to given `address`. Put `reason` in arguments.
    pub(super) fn send_error(&mut self, address: Address, reason: String) {
        use serde_json::json;
        let arguments = json!({ "reason": reason });
        let uuid = Self::uuid();
        let call = FunctionCall {
            uuid: format!("error_{}", uuid),
            target: Some(address),
            reply_to: None, // TODO: sure?
            module: None,
            fname: None,
            arguments,
            name: None,
            sender: self.config.local_address(),
        };
        self.call(call)
    }

    /// Add node to kademlia routing table.
    /// Node is identified by `node_id`, `addresses` and `public_key`
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

    /// Run kademlia bootstrap, to advertise ourselves in Kademlia
    pub fn bootstrap(&mut self) {
        use std::borrow::Borrow;
        // NOTE: Using Qm form of `peer_id` here (via peer_id.borrow), since kademlia uses that for keys
        self.kademlia
            .get_closest_peers(self.config.peer_id.borrow());
    }
}
