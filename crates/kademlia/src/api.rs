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

use std::convert::identity;

use futures::{
    channel::{mpsc::unbounded, oneshot},
    future::BoxFuture,
    FutureExt, StreamExt,
};
use libp2p::swarm::NetworkBehaviourAction;
use libp2p::{core::Multiaddr, PeerId};
use multihash::Multihash;

use fluence_libp2p::types::{Inlet, OneshotOutlet, Outlet};
use particle_protocol::Contact;

use crate::error::{KademliaError, Result};
use crate::Kademlia;

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
