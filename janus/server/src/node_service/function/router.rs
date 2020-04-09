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

use std::collections::{HashMap, HashSet, VecDeque};

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

use crate::{
    generate_swarm_event_type,
    misc::{SafeMultihash, WaitingQueue},
    node_service::{
        function::call::{Address, FunctionCall},
        function::ProtocolMessage,
    },
};
use failure::_core::time::Duration;
use parity_multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use trust_graph::TrustGraph;

mod behaviour;
mod provider;
mod relay;

pub type SwarmEventType = generate_swarm_event_type!(FunctionRouter);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BuiltinService {
    Provide { service_id: String },
}

impl BuiltinService {
    fn from_str(s: &str, arguments: serde_json::Value) -> Option<Self> {
        match s {
            "provide" => serde_json::from_value(arguments).ok(),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct LocalService {}

#[derive(Debug)]
enum WaitPeer {
    Found(FunctionCall),
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

/// Relay based on Kademlia. Responsibilities and mechanics:
/// - enqueues relay events, then async-ly searches Kademlia for destination nodes and then sends events
/// - all locally connected peers are stored in memory and periodically announced to Kademlia
/// - returns RelayEvent from poll
pub struct FunctionRouter {
    peer_id: PeerId,
    // Queue of events to send to the upper level.
    events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    kademlia: Kademlia<MemoryStore>,
    // Services provided by this node
    // TODO: health-check local services?
    // TODO: register these services
    local_services: HashMap<String, LocalService>, // ServiceId -> LocalService
    // TODO: clear queues on timeout?
    // Calls to services, waiting while provider of the service is found
    wait_provider: WaitingQueue<String, FunctionCall>, // ServiceId -> FunctionCall
    // Calls that are waiting while peer is found in the Kademlia
    wait_peer: WaitingQueue<PeerId, WaitPeer>, // PeerId -> FunctionCall
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
            .set_connection_idle_timeout(Duration::from_secs(2628000000)); // ~month
        let store = MemoryStore::new(peer_id.clone());
        let trust = TrustGraph::new(root_weights);

        Self {
            peer_id: peer_id.clone(),
            events: VecDeque::new(),
            kademlia: Kademlia::with_config(kp, peer_id, store, cfg, trust),
            wait_provider: WaitingQueue::new(),
            wait_peer: WaitingQueue::new(),
            local_services: HashMap::new(),
            connected_peers: HashSet::new(),
        }
    }

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
        if let Some(builtin) = BuiltinService::from_str(service.as_str(), call.arguments.clone()) {
            self.execute_builtin(builtin, call)
        } else if let Some(service) = self.local_services.get(&service) {
            self.execute_on_service(service, call)
        } else {
            self.find_service_provider(service, call);
        }
    }

    // Look for service providers, enqueue call to wait for providers
    fn find_service_provider(&mut self, service: String, call: FunctionCall) {
        self.wait_provider.enqueue(service.clone(), call);
        self.kademlia
            .get_providers(service.as_bytes().to_vec().into());
    }

    // Advance execution for calls waiting for this service: send them to first provider
    pub fn providers_found(&mut self, key: KademliaKey, providers: HashSet<PeerId>) {
        let key = String::from_utf8(key.as_ref().to_vec())
            .expect("Can't parse service key from kademlia key");

        let mut calls = self.wait_provider.remove(&key).peekable();
        // Check if calls are empty without actually advancing iterator
        if calls.peek().is_none() {
            log::warn!(
                "Providers found for {}, but there are no calls waiting for it",
                key
            );
        }

        // TODO: taking only first provider here. Should we send a call to all of them?
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

    // ####
    // ## Execution
    // ###

    // Send call to local service
    fn execute_on_service(&self, service: &LocalService, call: FunctionCall) {
        log::info!("Got call for local service {:?}: {:?}", service, call);
        // TODO: execute start_providing here if service is "provide"
        unimplemented!("Don't know how to execute service locally")
    }

    fn execute_builtin(&mut self, service: BuiltinService, call: FunctionCall) {
        match service {
            BuiltinService::Provide { service_id } => {
                println!("Would have published a service {}: {:?}", service_id, call)
            }
        }
    }

    fn execute_locally(&self, call: FunctionCall) {
        log::info!("Got call {:?}", call);
    }

    // ####
    // ## Finding peers
    // ###
    //
    // Look for peer in Kademlia, enqueue call to wait for result
    pub fn search_for_peer(&mut self, peer_id: PeerId, call: FunctionCall) {
        self.wait_peer
            .enqueue(peer_id.clone(), WaitPeer::Found(call));
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

    fn connected(&mut self, peer_id: PeerId) {
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
    fn disconnected(&mut self, peer_id: &PeerId) {
        log::info!(
            "Peer disconnected: {}. {} calls left waiting.",
            peer_id.to_base58(),
            self.wait_peer.count(peer_id)
        );
        self.connected_peers.remove(peer_id);

        // TODO: fail corresponding wait_peers? Though there shouldn't be any
    }

    fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connected_peers.contains(peer_id)
    }

    // Whether peer is in the routing table
    fn is_routable(&mut self, peer_id: &PeerId) -> bool {
        // TODO: Checking is_connected inside is_routable smells...
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
                self.execute_locally(call)
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
            Service { service } => {
                log::info!("Sending {} to service {}", &call.uuid, service);
                self.send_to_service(service, call)
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
                reply_to: None, // TODO: sure?
                arguments,
                ..call
            };
            self.send_to(reply_to, call)
        } else {
            log::warn!("Can't send error on call {:?}: reply_to is empty", call);
        }
    }
}
