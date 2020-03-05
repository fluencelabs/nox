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
use std::convert::TryInto;

use libp2p::core::either::EitherOutput;
use libp2p::kad::record::Key as KademliaKey;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::Kademlia;
use libp2p::kad::KademliaConfig;
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::PeerId;
use log::{debug, error};

// Reimport
pub use behaviour::*;

use crate::generate_swarm_event_type;
use crate::node_service::relay::RelayEvent;
use failure::_core::time::Duration;

mod behaviour;
mod events;
mod relay;

type SwarmEventType = generate_swarm_event_type!(KademliaRelay);

/// Relay based on Kademlia. Responsibilities and mechanics:
/// - enqueues relay events, then async-ly searches Kademlia for destination nodes and then sends events
/// - all locally connected peers are stored in memory and periodically announced to Kademlia
/// - returns RelayEvent from poll
pub struct KademliaRelay {
    // Queue of events to send to the upper level.
    events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    kademlia: Kademlia<MemoryStore>,
    // Enqueued RelayEvents, to be sent when providers are found
    messages: HashMap<PeerId, VecDeque<RelayEvent>>,
    // Locally connected peers
    peers: HashSet<PeerId>,
}

// TODO: move public methods to a trait
impl KademliaRelay {
    pub fn new(peer_id: PeerId) -> Self {
        let mut cfg = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5))
            .set_replication_factor(std::num::NonZeroUsize::new(5).unwrap());
        let store = MemoryStore::new(peer_id.clone());

        Self {
            events: VecDeque::new(),
            kademlia: Kademlia::with_config(peer_id, store, cfg),
            messages: HashMap::new(),
            peers: HashSet::new(),
        }
    }

    /// Try to relay `event` to a locally connected peer
    /// Returns `Some(event)` if peer was not found locally, `None` otherwise
    pub fn relay_local(&mut self, event: RelayEvent) -> Option<RelayEvent> {
        use libp2p::swarm::NetworkBehaviourAction::GenerateEvent;

        match PeerId::from_bytes(event.dst_id.clone()) {
            Ok(dst_peer) => {
                if !self.peers.contains(&dst_peer) {
                    // We were asked to relay a message to the local peer we don't have a connection to
                    return Some(event);
                }
                debug!("relay local to {}", dst_peer.to_base58());
                self.events.push_back(GenerateEvent(event));
            }
            Err(_err) => {
                error!("Error parsing PeerId from dst_id");
                return Some(event); // TODO: return error to the src_id
            }
        }

        None
    }

    /// Enqueue `event` to be sent, and start looking for nodes providing destination peer
    /// TODO: self.messages could grow without a limit
    pub fn relay_remote(&mut self, event: RelayEvent) {
        let dst_id = event.dst_id.clone();
        // TODO: Get rid of this Result or escalate error
        let dst_peer: PeerId = dst_id.try_into().expect("dst_id is not a correct PeerId");

        debug!("relay remote to {}", dst_peer.to_base58());

        self.messages
            .entry(dst_peer.clone())
            .or_insert_with(VecDeque::new)
            .push_back(event);

        // TODO: move peer_id => base58 => bytes serialization to a separate trait
        // base58 string is used here to mitigate equality issues of PeerId byte representation
        let key = dst_peer.to_base58().as_bytes().to_vec().into();
        self.kademlia.get_providers(key);
    }

    /// Announce to network that current node can route messages to peer of addr `peer_id`
    /// Schedules a recurring task; usually called on when new peer connected locally
    /// TODO: currently doesn't work on network of 2 nodes, works only on 3+
    pub fn announce_peer(&mut self, peer_id: PeerId) {
        debug!("Announcing peer {}", peer_id.to_base58());

        let key: KademliaKey = peer_id.to_base58().as_bytes().to_vec().into();
        self.kademlia.start_providing(key);

        self.peers.insert(peer_id);
    }

    /// Reverse of `announce_peer`: stop announcing the `peer_id`. Usually called on peer disconnection.
    pub fn bury_peer(&mut self, peer_id: &PeerId) {
        debug!("Buried peer {}", peer_id.to_base58());

        self.peers.remove(peer_id);

        let key: KademliaKey = peer_id.to_base58().as_bytes().to_vec().into();
        self.kademlia.stop_providing(&key);
    }

    /// Once providers for a `dst` peer are found, send enqueued `RelayEvent`s to them
    pub fn relay_to_providers(&mut self, dst: PeerId, providers: Vec<PeerId>) {
        for node in providers {
            if let Some(msg_queue) = self.messages.get_mut(&dst) {
                let events = msg_queue
                    .drain(..) // NOTE: clears out the queue
                    .map(|event| {
                        debug!(
                            "Relaying to peer {:?} via node {}",
                            dst.to_base58(),
                            node.to_base58()
                        );
                        NetworkBehaviourAction::SendEvent {
                            peer_id: node.clone(),
                            event: EitherOutput::First(event),
                        }
                    });
                self.events.extend(events);
            }
        }
    }
}
