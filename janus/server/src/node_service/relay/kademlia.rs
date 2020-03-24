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
use libp2p::swarm::NetworkBehaviour;
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::PeerId;

// Reimport
pub use behaviour::*;

use crate::generate_swarm_event_type;
use crate::node_service::relay::{Provider, RelayMessage};
use crate::peer_service::messages::ToPeerMsg;
use failure::_core::time::Duration;
use log::{debug, error};
use parity_multiaddr::Multiaddr;
use parity_multihash::Multihash;

mod behaviour;
mod events;
mod provider;
mod relay;

type SwarmEventType = generate_swarm_event_type!(KademliaRelay);

enum Promise {
    Relay(RelayMessage),
    FindProviders { client_id: PeerId, key: Multihash },
}

/// Represents a result of the enqueue_promise operation
pub enum Enqueued {
    // promise for such a key has already been in the queue
    New,
    // new promise created
    Existing,
}

/// Relay based on Kademlia. Responsibilities and mechanics:
/// - enqueues relay events, then async-ly searches Kademlia for destination nodes and then sends events
/// - all locally connected peers are stored in memory and periodically announced to Kademlia
/// - returns RelayEvent from poll
pub struct KademliaRelay {
    // Queue of events to send to the upper level.
    events: VecDeque<SwarmEventType>,
    // Underlying Kademlia node
    kademlia: Kademlia<MemoryStore>,
    // Enqueued promises, to be sent when providers are found
    promises: HashMap<Multihash, VecDeque<Promise>>, // TODO: is Multihash good-enough?
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
            promises: HashMap::new(),
            peers: HashSet::new(),
        }
    }

    // returns Enqueued::Existing if there is already promise for such provider
    fn enqueue_promise(&mut self, key: Multihash, promise: Promise) -> Enqueued {
        use std::collections::hash_map::Entry;
        use std::iter::FromIterator;

        match self.promises.entry(key) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(promise);
                Enqueued::Existing
            }
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::from_iter(std::iter::once(promise)));
                Enqueued::New
            }
        }
    }

    /// Try to relay `message` to a locally connected peer
    /// Returns `Some(event)` if peer was not found locally, `None` otherwise
    pub fn relay_local(&mut self, message: RelayMessage) -> Option<RelayMessage> {
        use libp2p::swarm::NetworkBehaviourAction::GenerateEvent;

        let deliver: Result<ToPeerMsg, _> = message.clone().try_into();

        // TODO: rewrite in a compact fashion
        match deliver {
            Ok(ToPeerMsg::Deliver {
                dst_id,
                src_id,
                data,
            }) => {
                if !self.peers.contains(&dst_id) {
                    // We were asked to relay a message to the local peer we don't have a connection to
                    Some(message)
                } else {
                    debug!("relay local to {}", dst_id.to_base58());
                    self.events.push_back(GenerateEvent(ToPeerMsg::Deliver {
                        dst_id,
                        src_id,
                        data,
                    }));
                    None
                }
            }
            Err(_) => {
                error!("Error parsing PeerId from dst_id");
                Some(message) // TODO: return error to the src_id
            }
            _ => Some(message),
        }
    }

    /// Enqueue `message` to be sent, and start looking for nodes providing destination peer
    /// TODO: self.messages could grow without a limit
    pub fn relay_remote(&mut self, message: RelayMessage) {
        let dst_id = message.dst_id.clone();
        // TODO: Get rid of this Result or escalate error
        let dst_peer: PeerId = dst_id.try_into().expect("dst_id is not a correct PeerId");

        debug!("relay remote to {}", dst_peer.to_base58());

        if let Enqueued::New =
            self.enqueue_promise(dst_peer.clone().into(), Promise::Relay(message))
        {
            // if there is no providers found in the queue - it needs to explicitly find them
            self.get_providers(dst_peer.into());
        }
    }

    /// Announce to network that current node can route messages to peer of addr `peer_id`
    /// Schedules a recurring task; usually called on when new peer connected locally
    /// TODO: currently doesn't work on network of 2 nodes, works only on 3+
    pub fn announce_peer(&mut self, peer_id: PeerId) {
        debug!("Announcing peer {}", peer_id.to_base58());

        self.provide(peer_id.clone().into());

        self.peers.insert(peer_id);
    }

    /// Reverse of `announce_peer`: stop announcing the `peer_id`. Usually called on peer disconnection.
    pub fn bury_peer(&mut self, peer_id: &PeerId) {
        debug!("Buried peer {}", peer_id.to_base58());

        self.peers.remove(peer_id);

        let key: KademliaKey = peer_id.to_base58().as_bytes().to_vec().into();
        self.kademlia.stop_providing(&key);
    }

    /// Signals to relay that providers are found, triggering completion of related promises
    pub fn providers_found(&mut self, key: KademliaKey, providers: Vec<PeerId>) {
        let key: Multihash = key
            .to_vec()
            .try_into()
            .expect("Can't parse key to multihash");

        let kademlia = &mut self.kademlia;

        if let Some(promises) = self.promises.get_mut(&key) {
            let events: Vec<SwarmEventType> = promises
                .drain(..)
                .map(|promise| match promise {
                    Promise::Relay(msg) => Self::generate_relay_events(msg, &providers),
                    Promise::FindProviders { client_id, key } => {
                        Self::generate_providers_events(kademlia, client_id, key, &providers)
                    }
                })
                .flatten()
                .collect();

            self.events.extend(events);
        }
    }

    fn generate_relay_events(message: RelayMessage, providers: &[PeerId]) -> Vec<SwarmEventType> {
        providers
            .iter()
            .map(|node| NetworkBehaviourAction::SendEvent {
                peer_id: node.clone(),
                event: EitherOutput::First(message.clone()),
            })
            .collect()
    }

    fn generate_providers_events(
        kademlia: &mut Kademlia<MemoryStore>,
        client: PeerId,
        key: Multihash,
        providers: &[PeerId],
    ) -> Vec<SwarmEventType> {
        let providers: Vec<(Multiaddr, PeerId)> = providers
            .iter()
            .map(|id| {
                kademlia
                    .addresses_of_peer(id)
                    .into_iter()
                    .map(move |addr| (addr, id.clone()))
            })
            .flatten()
            .collect();

        vec![NetworkBehaviourAction::GenerateEvent(
            ToPeerMsg::Providers {
                client_id: client,
                key,
                providers,
            },
        )]
    }

    pub fn get_providers(&mut self, key: Multihash) {
        self.kademlia.get_providers(key.into());
    }
}
