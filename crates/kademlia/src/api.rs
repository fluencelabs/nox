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
use crate::Kademlia;

use fluence_libp2p::generate_swarm_event_type;
use fluence_libp2p::types::{Inlet, OneshotOutlet, Outlet};
use particle_protocol::Contact;

use futures::{
    channel::{mpsc::unbounded, oneshot},
    future::BoxFuture,
    FutureExt, StreamExt,
};
use libp2p::{core::Multiaddr, identity::PublicKey, swarm::NetworkBehaviourEventProcess, PeerId};
use multihash::Multihash;
use std::convert::identity;

type Future<T> = BoxFuture<'static, T>;

pub trait KademliaApiT {
    fn bootstrap(&self) -> Future<Result<()>>;
    fn add_contact(&self, contact: Contact) -> bool;
    fn local_lookup(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>>;
    fn discover_peer(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>>;
    fn neighborhood(&self, key: Multihash, count: usize) -> Future<Result<Vec<PeerId>>>;
}

// marked `pub` to be available in benchmarks
#[derive(Debug)]
pub enum Command {
    AddContact {
        contact: Contact,
    },
    LocalLookup {
        peer: PeerId,
        out: OneshotOutlet<Vec<Multiaddr>>,
    },
    Bootstrap {
        out: OneshotOutlet<Result<()>>,
    },
    DiscoverPeer {
        peer: PeerId,
        out: OneshotOutlet<Result<Vec<Multiaddr>>>,
    },
    Neighborhood {
        key: Multihash,
        count: usize,
        out: OneshotOutlet<Result<Vec<PeerId>>>,
    },
}

pub type SwarmEventType = generate_swarm_event_type!(KademliaApiInlet);

#[derive(::libp2p::NetworkBehaviour)]
#[behaviour(poll_method = "custom_poll")]
pub struct KademliaApiInlet {
    #[behaviour(ignore)]
    inlet: Inlet<Command>,
    kademlia: Kademlia,
}

impl KademliaApiInlet {
    pub fn new(kademlia: Kademlia) -> (KademliaApi, Self) {
        let (outlet, inlet) = unbounded();
        let outlet = KademliaApi { outlet };
        (outlet, Self { inlet, kademlia })
    }

    pub fn add_addresses(
        &mut self,
        peer: PeerId,
        addresses: Vec<Multiaddr>,
        public_key: PublicKey,
    ) {
        self.kademlia.add_kad_node(peer, addresses, public_key)
    }

    fn execute(&mut self, cmd: Command) {
        match cmd {
            Command::AddContact { contact } => self.kademlia.add_contact(contact),
            Command::Bootstrap { out } => self.kademlia.bootstrap(out),
            Command::LocalLookup { peer, out } => self.kademlia.local_lookup(&peer, out),
            Command::DiscoverPeer { peer, out } => self.kademlia.discover_peer(peer, out),
            Command::Neighborhood { key, count, out } => {
                self.kademlia.neighborhood(key, count, out)
            }
        }
    }

    fn custom_poll(
        &mut self,
        cx: &mut std::task::Context,
        _: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<SwarmEventType> {
        use std::task::Poll;

        let mut wake = false;
        while let Poll::Ready(Some(cmd)) = self.inlet.poll_next_unpin(cx) {
            wake = true;
            self.execute(cmd)
        }

        if wake {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<()> for KademliaApiInlet {
    fn inject_event(&mut self, _: ()) {}
}

impl From<Kademlia> for (KademliaApi, KademliaApiInlet) {
    fn from(kademlia: Kademlia) -> Self {
        KademliaApiInlet::new(kademlia)
    }
}

#[derive(Clone, Debug)]
pub struct KademliaApi {
    // NOTE: marked `pub` to be available in benchmarks
    pub outlet: Outlet<Command>,
}

impl KademliaApi {
    fn execute<R, F>(&self, cmd: F) -> Future<Result<R>>
    where
        R: Send + Sync + 'static,
        F: FnOnce(OneshotOutlet<Result<R>>) -> Command,
    {
        let (out, inlet) = oneshot::channel();
        if self.outlet.unbounded_send(cmd(out)).is_err() {
            return futures::future::err(KademliaError::Cancelled).boxed();
        }
        inlet
            .map(|r| r.map_err(|_| KademliaError::Cancelled).and_then(identity))
            .boxed()
    }
}

impl KademliaApiT for KademliaApi {
    fn bootstrap(&self) -> Future<Result<()>> {
        self.execute(|out| Command::Bootstrap { out })
    }

    fn add_contact(&self, contact: Contact) -> bool {
        let cmd = Command::AddContact { contact };
        let send = self.outlet.unbounded_send(cmd);
        send.is_ok()
    }

    fn local_lookup(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>> {
        let (out, inlet) = oneshot::channel();
        let cmd = Command::LocalLookup { peer, out };
        let send = self.outlet.unbounded_send(cmd);
        if send.is_err() {
            return futures::future::err(KademliaError::Cancelled).boxed();
        }
        inlet
            .map(|r| r.map_err(|_| KademliaError::Cancelled))
            .boxed()
    }

    fn discover_peer(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>> {
        self.execute(|out| Command::DiscoverPeer { peer, out })
    }

    fn neighborhood(&self, key: Multihash, count: usize) -> Future<Result<Vec<PeerId>>> {
        self.execute(|out| Command::Neighborhood { key, count, out })
    }
}
