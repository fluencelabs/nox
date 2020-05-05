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

use super::builtin_service::BuiltinService;
use super::router::Service::Delegated;
use super::waiting_queues::WaitingQueues;
use faas_api::{Address, FunctionCall, ProtocolMessage};
use failure::_core::time::Duration;
use janus_libp2p::generate_swarm_event_type;
use janus_libp2p::SafeMultihash;
use libp2p::{
    core::either::EitherOutput,
    identity::{
        ed25519,
        ed25519::{Keypair, PublicKey},
    },
    kad::{
        record::Key as KademliaKey, store::MemoryStore, GetClosestPeersError, GetClosestPeersOk,
        GetClosestPeersResult, Kademlia, KademliaConfig,
    },
    swarm::{DialPeerCondition, NetworkBehaviour, NetworkBehaviourAction, NotifyHandler},
    PeerId,
};
use parity_multiaddr::Multiaddr;
use std::collections::{HashMap, HashSet, VecDeque};
use trust_graph::TrustGraph;

pub type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

#[derive(Debug, Clone)]
pub enum Service {
    Delegated { forward_to: Address },
}

/// Possible waiting states of the FunctionCall
#[derive(Debug)]
enum WaitPeer {
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
    peer_id: PeerId,
    // Queue of events to send to the upper level
    pub(super) events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    pub(super) kademlia: Kademlia<MemoryStore>,
    // Services provided by this node
    // TODO: health-check local services?
    // TODO: register these services
    local_services: HashMap<String, Service>, // ServiceId -> Service
    // TODO: clear queues on timeout?
    // Calls to services, waiting while provider of the service is found
    wait_provider: WaitingQueues<String, FunctionCall>, // ServiceId -> FunctionCall
    // Calls that are waiting for target peer of the call to change state (see WaitPeer)
    wait_peer: WaitingQueues<PeerId, WaitPeer>, // PeerId -> FunctionCall
    // TODO: clear connected_peers on inject_listener_closed?
    // Mediated by inject_connected & inject_disconnected
    connected_peers: HashSet<PeerId>,
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
    // ## Service routing
    // ###
    fn send_to_service(&mut self, service: String, call: FunctionCall) {
        if let Some(builtin) = BuiltinService::from(service.as_str(), call.arguments.clone()) {
            self.execute_builtin(builtin, call)
        } else if let Some(local_service) = self.local_services.get(&service).cloned() {
            log::info!(
                "Service {} was found locally. uuid {}",
                &service,
                &call.uuid
            );
            self.execute_on_service(local_service, call)
        } else {
            log::info!(
                "Service {} not found locally. uuid {}",
                &service,
                &call.uuid
            );
            self.find_service_provider(service, call);
        }
    }

    // Look for service providers, enqueue call to wait for providers
    fn find_service_provider(&mut self, service: String, call: FunctionCall) {
        self.wait_provider.enqueue(service.clone(), call);
        // TODO: don't call get_providers if there are already calls waiting for it
        self.kademlia
            .get_providers(service.as_bytes().to_vec().into());
    }

    // Advance execution for calls waiting for this service: send them to first provider
    pub fn providers_found(&mut self, key: KademliaKey, providers: HashSet<PeerId>) {
        let service_id = String::from_utf8(key.as_ref().to_vec())
            .expect("Can't parse service_id from kademlia key");

        log::info!(
            "Found {} providers for {}: {:?}",
            providers.len(),
            service_id,
            providers
        );

        let mut calls = self.wait_provider.remove(&service_id).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() && !providers.is_empty() {
            log::warn!(
                "Providers found for {}, but there are no calls waiting for it",
                service_id
            );
        }

        // TODO: taking only first provider here. Should we send a call to all of them?
        // TODO: weight providers according to TrustGraph
        if let Some(provider) = providers.into_iter().next() {
            for call in calls {
                self.send_call(provider.clone(), call);
            }
        } else {
            for call in calls {
                self.send_error_on_call(call, "No providers found".to_string());
            }
        }
    }

    // Removes all services that are delegated by specified PeerId
    fn remove_delegated_services(&mut self, delegate: &PeerId) {
        let delegate = delegate.to_base58();

        self.local_services.retain(|k, v| match v {
            Delegated { forward_to } => {
                let to_delegate = forward_to
                    .destination_peer()
                    .map_or(false, |p| p.to_base58() == delegate);
                if to_delegate {
                    log::info!(
                        "Removing delegated service {}. forward_to: {:?}, delegate: {}",
                        k,
                        forward_to,
                        delegate
                    );
                }
                !to_delegate
            }
        });
    }

    // ####
    // ## Execution
    // ###

    // Send call to local service
    fn execute_on_service(&mut self, service: Service, call: FunctionCall) {
        log::debug!("Got call for local service {:?}: {:?}", service, call);
        match service {
            Delegated { forward_to } => {
                log::info!("Forwarding service call {} to {}", call.uuid, forward_to);
                self.send_to(forward_to, call)
            }
        }
    }

    fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        match service {
            BuiltinService::DelegateProviding { service_id } => match &call.reply_to {
                // To avoid routing cycle (see https://gist.github.com/folex/61700dd6afa14fbe3d1168e04dfe2661 for bug postmortem)
                Some(Address::Relay { relay, .. }) if !self.is_local(&relay) => {
                    log::warn!("Decline to register peer for a different relay: {:?}", call);
                    self.send_error_on_call(
                        call,
                        "declined to register peer for a different relay".into(),
                    );
                }
                // Happy path – registering delegated service. TODO: is it sound to allow forward_to = Address::Service?
                Some(forward_to) => {
                    let new = Delegated {
                        forward_to: forward_to.clone(),
                    };
                    let replaced = self.local_services.insert(service_id.clone(), new.clone());
                    if let Some(replaced) = replaced {
                        log::warn!(
                            "Replacing service {:?} with {:?} due to call {}",
                            replaced,
                            new,
                            &call.uuid
                        );
                    }
                    self.kademlia
                        .start_providing(service_id.as_bytes().to_vec().into());
                    log::info!("Published a service {}: {:?}", service_id, call)
                }
                // If there's no `reply_to`, then we don't know where to forward, so can't register
                None => {
                    log::warn!("reply_to was not defined in {:?}", call);
                    self.send_error_on_call(
                        call,
                        "reply_to must be defined when calling 'provide' service".into(),
                    )
                }
            },
        }
    }

    fn handle_local_call(&mut self, call: FunctionCall) {
        log::info!("Got call {:?}, don't know what to do with that", call);
        self.send_error_on_call(call, "Don't know how to handle that".to_string())
    }

    // ####
    // ## Finding peers
    // ###
    //
    // Look for peer in Kademlia, enqueue call to wait for result
    pub fn search_for_peer(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.wait_peer
            .enqueue(peer_id.clone(), WaitPeer::Found(call));
        // TODO: don't call get_closest_peers if there are already some calls waiting for it
        self.kademlia
            .get_closest_peers(SafeMultihash::from(peer_id))
    }

    // Take all calls waiting for this peer to be found, and send them
    pub fn found_closest(&mut self, result: GetClosestPeersResult) {
        let (key, peers) = match result {
            Ok(GetClosestPeersOk { key, peers }) => (key, peers),
            Err(GetClosestPeersError::Timeout { key, peers }) => {
                let will_try = if !peers.is_empty() {
                    " Will try to send calls anyway"
                } else {
                    ""
                };
                log::error!(
                    "Timed out while finding closest peer {}. Found only {} peers.{}",
                    bs58::encode(&key).into_string(),
                    peers.len(),
                    will_try
                );
                (key, peers)
            }
        };

        let peer_id = match PeerId::from_bytes(key) {
            Err(err) => {
                log::error!(
                    "Can't parse peer id from GetClosestPeersResult key: {}",
                    bs58::encode(err).into_string()
                );
                return;
            }
            Ok(peer_id) => peer_id,
        };

        let calls = self
            .wait_peer
            .remove_with(&peer_id, |wp| matches!(wp, WaitPeer::Found(_)));
        // TODO: is `peers.contains` necessary? Not sure
        // Check if peer is in the routing table.
        let routable = peers.contains(&peer_id) || self.is_routable(&peer_id);
        if routable {
            for call in calls {
                // Send the message once peer is connected
                self.send_to_routable(peer_id.clone(), call.call())
            }
        } else {
            for call in calls {
                // Report error
                self.send_error_on_call(
                    call.call(),
                    "Peer wasn't found via GetClosestPeers".to_string(),
                )
            }
        }
    }

    fn connect(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.wait_peer
            .enqueue(peer_id.clone(), WaitPeer::Connected(call));

        log::info!("Dialing {}", peer_id.to_base58());
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id,
            condition: DialPeerCondition::Disconnected,
        });
    }

    pub(super) fn connected(&mut self, peer_id: PeerId) {
        log::info!("Peer connected: {}", peer_id.to_base58());
        self.connected_peers.insert(peer_id.clone());

        let waiting = self
            .wait_peer
            .remove_with(&peer_id, |wp| matches!(wp, WaitPeer::Connected(_)));

        // TODO: leave move or remove move from closure?
        waiting.for_each(move |wp| match wp {
            WaitPeer::Connected(call) => self.send_to_connected(peer_id.clone(), call),
            WaitPeer::Found(_) => unreachable!("Can't happen. Just filtered WaitPeer::Connected"),
        });
    }

    // TODO: clear connected_peers on inject_listener_closed?
    pub(super) fn disconnected(&mut self, peer_id: &PeerId) {
        log::info!(
            "Peer disconnected: {}. {} calls left waiting.",
            peer_id.to_base58(),
            self.wait_peer.count(&peer_id)
        );
        self.connected_peers.remove(peer_id);

        self.remove_delegated_services(peer_id);

        let waiting_calls = self.wait_peer.remove(peer_id);
        for waiting in waiting_calls.into_iter() {
            self.send_error_on_call(waiting.call(), "Peer disconnected".into());
        }
    }

    fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains(peer_id)
    }

    // Whether peer is in the routing table
    fn is_routable(&mut self, peer_id: &PeerId) -> bool {
        // TODO: Checking `is_connected` inside `is_routable` smells...
        let connected = self.is_connected(peer_id);

        let kad = self.kademlia.addresses_of_peer(peer_id);
        let in_kad = !kad.is_empty();

        log::debug!(
            "peer {} in routing table: Connected? {} Kademlia {:?}",
            peer_id.to_base58(),
            connected,
            kad
        );
        connected || in_kad
    }

    // Whether given peer id is equal to ours
    fn is_local(&self, peer_id: &PeerId) -> bool {
        let me = self.peer_id.to_base58();
        let they = peer_id.to_base58();
        if me == they {
            log::debug!("{} is ME! (equals to {})", they, me);
            true
        } else {
            log::debug!("{} is THEM! (not equal to {})", they, me);
            false
        }
    }

    // ####
    // ## Sending calls
    // ###

    // Send call to any address
    fn send_to(&mut self, address: Address, call: FunctionCall) {
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
    fn send_call(&mut self, to: PeerId, call: FunctionCall) {
        if self.is_routable(&to) {
            self.send_to_routable(to, call);
        } else {
            self.search_for_peer(to, call);
        }
    }

    fn send_to_routable(&mut self, to: PeerId, call: FunctionCall) {
        if self.is_connected(&to) {
            self.send_to_connected(to, call)
        } else {
            self.connect(to, call)
        }
    }

    // Schedule sending call to the peer, assuming peer is connected
    fn send_to_connected(&mut self, peer_id: PeerId, call: FunctionCall) {
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
    fn send_error_on_call(&mut self, mut call: FunctionCall, reason: String) {
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
