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
use fluence_libp2p::{generate_swarm_event_type, types::OneshotOutlet};
use particle_protocol::Contact;

use futures::FutureExt;
use futures_timer::Delay;
use libp2p::identity::PublicKey;
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::{
    core::Multiaddr,
    identity,
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
use std::convert::TryFrom;
use std::{
    cmp::min,
    collections::HashMap,
    ops::Deref,
    task::Waker,
    time::{Duration, Instant},
};

pub struct KademliaConfig {
    pub peer_id: PeerId,
    pub keypair: identity::Keypair,
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
    created: Instant,
}

impl PendingPeer {
    pub fn new(out: OneshotOutlet<Result<Vec<Multiaddr>>>) -> Self {
        Self {
            out,
            created: Instant::now(),
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

// type SwarmEventType = generate_swarm_event_type!(Kademlia);
type SwarmEventType =
    NetworkBehaviourAction<(), <Kademlia as libp2p::swarm::NetworkBehaviour>::ProtocolsHandler>;

#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll", event_process = true)]
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
    #[behaviour(ignore)]
    // Timer to track timed out requests, and return errors ASAP
    timer: Delay,
}

impl Kademlia {
    pub fn new(config: KademliaConfig, registry: Option<&Registry>) -> Self {
        let timer = Delay::new(config.query_timeout);

        let store = MemoryStore::new(config.peer_id);
        let mut kademlia = kad::Kademlia::with_config(config.peer_id, store, config.as_libp2p());

        // if let Some(registry) = registry {
        //     kademlia.enable_metrics(registry);
        // }

        Self {
            kademlia,
            queries: <_>::default(),
            pending_peers: <_>::default(),
            failed_peers: <_>::default(),
            config,
            waker: None,
            timer,
        }
    }

    pub fn add_kad_node(&mut self, peer: PeerId, addresses: Vec<Multiaddr>, public_key: PublicKey) {
        for addr in addresses {
            self.kademlia.add_address(&peer, addr.clone());
        }
        self.wake();
    }
}

impl Kademlia {
    pub fn add_contact(&mut self, contact: Contact) {
        debug_assert!(!contact.addresses.is_empty(), "no addresses in contact");

        for addr in contact.addresses {
            self.kademlia.add_address(&contact.peer_id, addr);
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

        let pending = PendingPeer::new(outlet);
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

    pub fn neighborhood(
        &mut self,
        key: Multihash,
        count: usize,
        outlet: OneshotOutlet<Result<Vec<PeerId>>>,
    ) {
        let key = key.into();
        let peers = self.kademlia.get_closest_local_peers(&key);
        let peers = peers.take(count);
        let peers = peers.map(|p| p.into_preimage());
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
        if self.pending_peers.is_empty() && self.failed_peers.is_empty() {
            return Poll::Pending;
        };

        let failed_peers = &mut self.failed_peers;
        let config = self.config.deref();
        // timer will wake up current task after `next_wake`
        let mut next_wake = min(config.query_timeout, config.ban_cooldown);
        let now = Instant::now();

        // Remove empty keys
        self.pending_peers.retain(|id, peers| {
            // remove expired
            let expired = peers.drain_filter(|p| {
                has_timed_out(now, p.created, config.query_timeout, &mut next_wake)
            });

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

        self.failed_peers.retain(|_, failed| {
            if let Some(ban) = failed.ban {
                // unban (remove) a peer if cooldown has passed
                let unban = has_timed_out(now, ban, config.ban_cooldown, &mut next_wake);
                if unban {
                    return false;
                }
            }

            // ban peers with too many failures
            if failed.count >= config.peer_fail_threshold {
                failed.ban = Some(now);
            }

            true
        });

        // task will be awaken after `next_wake`
        self.timer.reset(next_wake);
        // register current task within timer
        self.timer.poll_unpin(cx).is_ready(); // `is_ready` here is to avoid "must use" warning

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

/// Calculate whether some entity has reached its timeout.
/// `now` - current time
/// `timestamp` - starting point
/// `timeout` - after what time it should be marked as timed out
/// `wake` - if entity has not timed out, when to wake  
fn has_timed_out(now: Instant, timestamp: Instant, timeout: Duration, wake: &mut Duration) -> bool {
    let elapsed = now.duration_since(timestamp);
    // how much has passed of the designated timeout
    match timeout.checked_sub(elapsed) {
        // didn't reach timeout yet
        Some(wake_after) if !wake_after.is_zero() => {
            // wake up earlier, if needed
            *wake = min(wake_after, *wake);
            false
        }
        // timed out
        _ => true,
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for Kademlia {
    fn inject_event(&mut self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryCompleted { id, result, .. } => match result {
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
            KademliaEvent::InboundRequest { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kademlia;
    use crate::{KademliaConfig, KademliaError};
    use async_std::task;
    use fluence_libp2p::{build_memory_transport, RandomPeerId};

    use fluence_libp2p::random_multiaddr::create_memory_maddr;
    use futures::channel::oneshot;
    use futures::StreamExt;
    use libp2p::core::Multiaddr;
    use libp2p::identity::{Keypair, PublicKey};
    use libp2p::PeerId;
    use libp2p::Swarm;
    use std::task::Poll;
    use std::time::Duration;

    fn kad_config() -> KademliaConfig {
        let keypair = Keypair::generate_ed25519();
        let public_key = keypair.public();
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
        let config = kad_config();
        let kp = config.keypair.clone();
        let peer_id = config.peer_id.clone();
        let pk = config.keypair.public();
        let kad = Kademlia::new(config, None);
        let timeout = Duration::from_secs(20);

        let mut swarm = Swarm::new(build_memory_transport(kp, timeout), kad, peer_id);
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
        Swarm::dial(&mut a, b_addr.clone()).unwrap();
        Swarm::dial(&mut a, c_addr.clone()).unwrap();
        Swarm::dial(&mut a, d_addr.clone()).unwrap();
        Swarm::dial(&mut a, e_addr.clone()).unwrap();
        a.behaviour_mut()
            .kademlia
            .add_address(Swarm::local_peer_id(&b), b_addr.clone());
        a.behaviour_mut()
            .kademlia
            .add_address(Swarm::local_peer_id(&c), c_addr.clone());
        a.behaviour_mut()
            .kademlia
            .add_address(Swarm::local_peer_id(&d), d_addr.clone());
        a.behaviour_mut()
            .kademlia
            .add_address(Swarm::local_peer_id(&e), e_addr.clone());
        a.behaviour_mut().kademlia.bootstrap().ok();

        // b knows only a, wants to discover c
        Swarm::dial(&mut b, a_addr.clone()).unwrap();
        b.behaviour_mut()
            .kademlia
            .add_address(Swarm::local_peer_id(&a), a_addr);
        let (out, inlet) = oneshot::channel();
        b.behaviour_mut()
            .discover_peer(Swarm::local_peer_id(&c).clone(), out);
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

        node.behaviour_mut()
            .discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.behaviour().queries.len(), 1);
        node.behaviour_mut()
            .discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.behaviour().queries.len(), 1);
    }

    #[test]
    fn ban() {
        use async_std::future::timeout;

        let (mut node, _, _) = make_node();
        let peer = RandomPeerId::random();

        node.behaviour_mut()
            .discover_peer(peer, oneshot::channel().0);
        assert_eq!(node.behaviour_mut().queries.len(), 1);

        task::block_on(timeout(Duration::from_millis(200), node.select_next_some())).ok();
        assert_eq!(node.behaviour_mut().failed_peers.len(), 1);
        assert!(node
            .behaviour_mut()
            .failed_peers
            .get(&peer)
            .unwrap()
            .ban
            .is_some());

        let (out, inlet) = oneshot::channel();
        node.behaviour_mut().discover_peer(peer, out);

        let banned = task::block_on(timeout(Duration::from_millis(200), inlet))
            .unwrap()
            .unwrap();
        assert!(matches!(banned, Err(KademliaError::PeerBanned)));
    }
}
