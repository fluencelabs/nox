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

use crate::error::{KademliaError, Result};

use control_macro::get_return;
use fluence_libp2p::generate_swarm_event_type;
use fluence_libp2p::types::OneshotOutlet;
use particle_protocol::Contact;
use trust_graph::InMemoryStorage;

use libp2p::identity::PublicKey;
use libp2p::{
    core::Multiaddr,
    identity::{ed25519, ed25519::Keypair},
    kad::{
        self, store::MemoryStore, BootstrapError, BootstrapOk, BootstrapResult,
        GetClosestPeersError, GetClosestPeersOk, GetClosestPeersResult, KademliaEvent, QueryId,
        QueryResult,
    },
    swarm::{NetworkBehaviour, NetworkBehaviourEventProcess},
    PeerId,
};
use multihash::Multihash;
use prometheus::Registry;
use std::ops::Deref;
use std::task::Waker;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub struct KademliaConfig {
    pub peer_id: PeerId,
    pub keypair: Keypair,
    // TODO: wonderful name clashing. I guess it is better to rename one of the KademliaConfig's to something else. You'll figure it out.
    pub kad_config: server_config::KademliaConfig,
}

impl Deref for KademliaConfig {
    type Target = server_config::KademliaConfig;

    fn deref(&self) -> &Self::Target {
        &self.kad_config
    }
}

#[derive(Debug)]
pub enum PendingQuery {
    Peer(PeerId),
    Neighborhood(OneshotOutlet<Result<Vec<PeerId>>>),
    Unit(OneshotOutlet<Result<()>>),
}

#[derive(Debug)]
struct PendingPeer {
    out: OneshotOutlet<Result<Vec<Multiaddr>>>,
    deadline: Instant,
}

impl PendingPeer {
    pub fn new(out: OneshotOutlet<Result<Vec<Multiaddr>>>, timeout: Duration) -> Self {
        Self {
            out,
            deadline: Instant::now() + timeout,
        }
    }
}

#[derive(Default, Debug)]
struct FailedPeer {
    /// When the peer was banned
    pub ban: Option<Instant>,
    /// How many times we failed to discover the peer
    pub count: usize,
}

impl FailedPeer {
    pub fn increment(&mut self) {
        self.count += 1;
    }
}

type SwarmEventType = generate_swarm_event_type!(Kademlia);
type TrustGraph = trust_graph::TrustGraph<InMemoryStorage>;

#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct Kademlia {
    kademlia: kad::Kademlia<MemoryStore>,

    #[behaviour(ignore)]
    queries: HashMap<QueryId, PendingQuery>,
    #[behaviour(ignore)]
    pending_peers: HashMap<PeerId, Vec<PendingPeer>>,
    #[behaviour(ignore)]
    failed_peers: HashMap<PeerId, FailedPeer>,
    #[behaviour(ignore)]
    config: KademliaConfig,
    #[behaviour(ignore)]
    waker: Option<Waker>,
}

impl Kademlia {
    pub fn new(
        config: KademliaConfig,
        trust_graph: TrustGraph,
        registry: Option<&Registry>,
    ) -> Self {
        let store = MemoryStore::new(config.peer_id);

        let mut kademlia = kad::Kademlia::with_config(
            config.keypair.clone(),
            config.peer_id,
            store,
            config.as_libp2p(),
            trust_graph,
        );

        if let Some(registry) = registry {
            kademlia.enable_metrics(registry);
        }

        Self {
            kademlia,
            queries: <_>::default(),
            pending_peers: <_>::default(),
            failed_peers: <_>::default(),
            config,
            waker: None,
        }
    }

    pub fn add_kad_node(
        &mut self,
        peer: PeerId,
        addresses: Vec<Multiaddr>,
        public_key: ed25519::PublicKey,
    ) {
        for addr in addresses {
            self.kademlia
                .add_address(&peer, addr.clone(), public_key.clone());
        }
        self.wake();
    }
}

impl Kademlia {
    pub fn add_contact(&mut self, contact: Contact) {
        debug_assert!(!contact.addresses.is_empty(), "no addresses in contact");

        let pk = contact.peer_id.as_public_key();
        debug_assert!(pk.is_some(), "peer id must contain public key");

        let pk = match pk {
            Some(PublicKey::Ed25519(pk)) => pk,
            Some(pk) => {
                log::error!("Unsupported key type {:?} on {}", pk, contact);
                return;
            }
            None => {
                log::error!("No public key in the peer id of contact {}", contact);
                return;
            }
        };
        for addr in contact.addresses {
            self.kademlia
                .add_address(&contact.peer_id, addr, pk.clone());
        }
    }

    pub fn bootstrap(&mut self, outlet: OneshotOutlet<Result<()>>) {
        if let Ok(query_id) = self.kademlia.bootstrap() {
            self.queries.insert(query_id, PendingQuery::Unit(outlet));
            self.wake();
        } else {
            outlet.send(Err(KademliaError::NoKnownPeers)).ok();
        }
    }

    pub fn local_lookup(&mut self, peer: &PeerId, outlet: OneshotOutlet<Vec<Multiaddr>>) {
        outlet.send(self.kademlia.addresses_of_peer(peer)).ok();
    }

    pub fn discover_peer(&mut self, peer: PeerId, outlet: OneshotOutlet<Result<Vec<Multiaddr>>>) {
        let local = self.kademlia.addresses_of_peer(&peer);
        if !local.is_empty() {
            outlet.send(Ok(local)).ok();
            return;
        }
        if self.is_banned(&peer) {
            outlet.send(Err(KademliaError::PeerBanned)).ok();
            return;
        }

        let pending = PendingPeer::new(outlet, self.config.query_timeout);
        let outlets = self.pending_peers.entry(peer).or_default();
        // If there are existing outlets, then discovery process is already running
        let discovering = !outlets.is_empty();
        // Subscribe on discovery result
        outlets.push(pending);

        // Run discovery only if there's no discovery already running
        if !discovering {
            let query_id = self.kademlia.get_closest_peers(peer);
            self.queries.insert(query_id, PendingQuery::Peer(peer));
            self.wake();
        }
    }

    pub fn neighborhood(&mut self, key: Multihash, outlet: OneshotOutlet<Result<Vec<PeerId>>>) {
        let peers = self.kademlia.local_closest_peers(key);
        let peers = peers.into_iter().map(|p| p.peer_id.into_preimage());
        outlet.send(Ok(peers.collect())).ok();
        self.wake();
    }

    pub fn remote_neighborhood(
        &mut self,
        key: Multihash,
        outlet: OneshotOutlet<Result<Vec<PeerId>>>,
    ) {
        let query_id = self.kademlia.get_closest_peers(key);
        self.queries
            .insert(query_id, PendingQuery::Neighborhood(outlet));
        self.wake();
    }
}

impl Kademlia {
    fn peer_discovered(&mut self, peer: PeerId, addresses: Vec<Multiaddr>) {
        log::trace!(
            target: "network",
            "discovered peer {} with {:?} addresses",
            peer,
            addresses,
        );

        if let Some(pendings) = self.pending_peers.remove(&peer) {
            for pending in pendings {
                pending.out.send(Ok(addresses.clone())).ok();
            }
        }

        // unban peer
        self.failed_peers.remove(&peer);
    }

    fn closest_finished(&mut self, id: QueryId, result: GetClosestPeersResult) {
        use GetClosestPeersError::Timeout;

        match get_return!(self.queries.remove(&id)) {
            PendingQuery::Peer(peer_id) => {
                let addresses = self.kademlia.addresses_of_peer(&peer_id);
                // if addresses are empty - do nothing, let it be finished by timeout;
                // motivation: more addresses might appear later through other events
                if !addresses.is_empty() {
                    self.peer_discovered(peer_id, addresses)
                }
            }
            PendingQuery::Neighborhood(outlet) => {
                let result = match result {
                    Ok(GetClosestPeersOk { peers, .. }) if !peers.is_empty() => Ok(peers),
                    Ok(GetClosestPeersOk { .. }) => Err(KademliaError::NoPeersFound),
                    Err(Timeout { peers, .. }) if !peers.is_empty() => Ok(peers),
                    Err(Timeout { .. }) => Err(KademliaError::Timeout),
                };
                outlet.send(result).ok();
            }
            PendingQuery::Unit(outlet) => {
                outlet.send(Ok(())).ok();
            }
        }
    }

    fn bootstrap_finished(&mut self, id: QueryId, result: BootstrapResult) {
        // how many buckets there are left to try
        let num_remaining = match result {
            Ok(BootstrapOk { num_remaining, .. }) => Some(num_remaining),
            Err(BootstrapError::Timeout { num_remaining, .. }) => num_remaining,
        };

        // if all desired buckets were tried, signal bootstrap completion
        // note that it doesn't care about successes or errors; that's because bootstrap keeps
        // going through next buckets even if it failed on previous buckets
        if num_remaining == Some(0) {
            if let Some(PendingQuery::Unit(outlet)) = self.queries.remove(&id) {
                outlet.send(Ok(())).ok();
            }
        }
    }

    fn custom_poll(
        &mut self,
        cx: &mut std::task::Context,
        _: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<SwarmEventType> {
        use std::task::Poll;

        self.waker = Some(cx.waker().clone());

        // Exit early to avoid Instant::now calculation
        if self.pending_peers.is_empty() {
            return Poll::Pending;
        };

        let now = Instant::now();
        let failed_peers = &mut self.failed_peers;
        // Remove empty keys
        self.pending_peers.retain(|id, peers| {
            // remove expired
            let expired = peers.drain_filter(|p| p.deadline <= now);

            let mut timed_out = false;
            for p in expired {
                timed_out = true;
                // notify expired
                p.out.send(Err(KademliaError::Timeout)).ok();
            }
            // count failure if there was at least 1 timeout
            if timed_out {
                failed_peers.entry(*id).or_default().increment();
            }

            // empty entries will be removed
            !peers.is_empty()
        });

        let config = self.config.deref();
        self.failed_peers.retain(|_, failed| {
            if let Some(ban) = failed.ban {
                if now.duration_since(ban) >= config.ban_cooldown {
                    // unban (remove) a peer if cooldown has passed
                    return false;
                }
            }

            // ban peers with too many failures
            if failed.count >= config.peer_fail_threshold {
                failed.ban = Some(now);
            }

            true
        });

        // NOTE: task will not be awaken until something happens;
        //       that implies that timeouts are of low resolution
        Poll::Pending
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref()
        }
    }

    fn is_banned(&self, peer: &PeerId) -> bool {
        self.failed_peers
            .get(peer)
            .map_or(false, |f| f.ban.is_some())
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Kademlia {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::QueryResult { id, result, .. } => match result {
                QueryResult::GetClosestPeers(result) => self.closest_finished(id, result),
                QueryResult::Bootstrap(result) => self.bootstrap_finished(id, result),
                _ => {}
            },
            KademliaEvent::UnroutablePeer { .. } => {}
            KademliaEvent::RoutingUpdated {
                peer, addresses, ..
            } => self.peer_discovered(peer, addresses.into_vec()),
            KademliaEvent::RoutablePeer { peer, address }
            | KademliaEvent::PendingRoutablePeer { peer, address } => {
                self.peer_discovered(peer, vec![address])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kademlia;
    use crate::{KademliaConfig, KademliaError};
    use async_std::task;
    use fluence_libp2p::{build_memory_transport, RandomPeerId};
    use futures::channel::oneshot;
    use futures::StreamExt;
    use libp2p::core::Multiaddr;
    use libp2p::identity::ed25519::{Keypair, PublicKey};
    use libp2p::identity::PublicKey::Ed25519;
    use libp2p::PeerId;
    use libp2p::Swarm;
    use std::task::Poll;
    use std::time::Duration;
    use test_utils::create_memory_maddr;
    use trust_graph::{InMemoryStorage, TrustGraph};

    fn kad_config() -> KademliaConfig {
        let keypair = Keypair::generate();
        let public_key = Ed25519(keypair.public());
        let peer_id = PeerId::from(public_key);

        KademliaConfig {
            peer_id,
            keypair,
            kad_config: server_config::KademliaConfig {
                query_timeout: Duration::from_millis(100),
                peer_fail_threshold: 1,
                ban_cooldown: Duration::from_secs(1),
                ..Default::default()
            },
        }
    }

    fn make_node() -> (Swarm<Kademlia>, Multiaddr, PublicKey) {
        let trust_graph = TrustGraph::new(InMemoryStorage::new());
        let config = kad_config();
        let kp = libp2p::identity::Keypair::Ed25519(config.keypair.clone());
        let peer_id = config.peer_id.clone();
        let pk = config.keypair.public();
        let kad = Kademlia::new(config, trust_graph, None);

        let mut swarm = Swarm::new(build_memory_transport(kp), kad, peer_id);
        let maddr = create_memory_maddr();
        Swarm::listen_on(&mut swarm, maddr.clone()).ok();

        (swarm, maddr, pk)
    }

    #[test]
    fn discovery() {
        let (mut a, a_addr, a_pk) = make_node();
        let (mut b, b_addr, b_pk) = make_node();
        let (c, c_addr, c_pk) = make_node();
        let (d, d_addr, d_pk) = make_node();
        let (e, e_addr, e_pk) = make_node();

        // a knows everybody
        Swarm::dial_addr(&mut a, b_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, c_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, d_addr.clone()).unwrap();
        Swarm::dial_addr(&mut a, e_addr.clone()).unwrap();
        a.kademlia
            .add_address(Swarm::local_peer_id(&b), b_addr.clone(), b_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&c), c_addr.clone(), c_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&d), d_addr.clone(), d_pk);
        a.kademlia
            .add_address(Swarm::local_peer_id(&e), e_addr.clone(), e_pk);
        a.kademlia.bootstrap().ok();

        // b knows only a, wants to discover c
        Swarm::dial_addr(&mut b, a_addr.clone()).unwrap();
        b.kademlia
            .add_address(Swarm::local_peer_id(&a), a_addr, a_pk);
        let (out, inlet) = oneshot::channel();
        b.discover_peer(Swarm::local_peer_id(&c).clone(), out);
        let discover_fut = inlet;

        let maddr = async_std::task::block_on(async move {
            let mut swarms = vec![a, b, c, d, e];
            task::spawn(futures::future::poll_fn(move |ctx| {
                for (_, swarm) in swarms.iter_mut().enumerate() {
                    if let Poll::Ready(Some(_)) = swarm.poll_next_unpin(ctx) {
                        return Poll::Ready(());
                    }
                }
                Poll::Pending
            }));

            discover_fut.await
        });

        assert_eq!(maddr.unwrap().unwrap()[0], c_addr);
    }

    #[test]
    fn dont_repeat_discovery() {
        let (mut node, _, _) = make_node();
        let peer = RandomPeerId::random();

        node.discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.queries.len(), 1);
        node.discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.queries.len(), 1);
    }

    #[test]
    fn ban() {
        use async_std::future::timeout;

        let (mut node, _, _) = make_node();
        let peer = RandomPeerId::random();

        node.discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.queries.len(), 1);

        task::block_on(timeout(Duration::from_millis(200), node.select_next_some())).ok();
        assert_eq!(node.failed_peers.len(), 1);
        assert!(node.failed_peers.get(&peer).unwrap().ban.is_some());

        let (out, inlet) = oneshot::channel();
        node.discover_peer(peer, out);

        let banned = task::block_on(timeout(Duration::from_millis(200), inlet))
            .unwrap()
            .unwrap();
        assert!(matches!(banned, Err(KademliaError::PeerBanned)));
    }
}
