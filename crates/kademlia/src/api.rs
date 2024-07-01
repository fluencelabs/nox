/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::convert::identity;

use futures::{future::BoxFuture, FutureExt};
use libp2p::{core::Multiaddr, PeerId};
use multihash::Multihash;
use particle_protocol::Contact;
use tokio::sync::{mpsc, oneshot};

use crate::error::{KademliaError, Result};

type Future<T> = BoxFuture<'static, T>;

pub trait KademliaApiT {
    fn bootstrap(&self) -> Future<Result<()>>;
    fn add_contact(&self, contact: Contact) -> bool;
    fn local_lookup(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>>;
    fn discover_peer(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>>;
    fn neighborhood(&self, key: Multihash<64>, count: usize) -> Future<Result<Vec<PeerId>>>;
}

// marked `pub` to be available in benchmarks
#[derive(Debug)]
pub enum Command {
    AddContact {
        contact: Contact,
    },
    LocalLookup {
        peer: PeerId,
        out: oneshot::Sender<Vec<Multiaddr>>,
    },
    Bootstrap {
        out: oneshot::Sender<Result<()>>,
    },
    DiscoverPeer {
        peer: PeerId,
        out: oneshot::Sender<Result<Vec<Multiaddr>>>,
    },
    Neighborhood {
        key: Multihash<64>,
        count: usize,
        out: oneshot::Sender<Result<Vec<PeerId>>>,
    },
}

#[derive(Clone, Debug)]
pub struct KademliaApi {
    // NOTE: marked `pub` to be available in benchmarks
    pub outlet: mpsc::UnboundedSender<Command>,
}

impl KademliaApi {
    fn execute<R, F>(&self, cmd: F) -> Future<Result<R>>
    where
        R: Send + Sync + 'static,
        F: FnOnce(oneshot::Sender<Result<R>>) -> Command,
    {
        let (out, inlet) = oneshot::channel();
        if self.outlet.send(cmd(out)).is_err() {
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
        let send = self.outlet.send(cmd);
        send.is_ok()
    }

    fn local_lookup(&self, peer: PeerId) -> Future<Result<Vec<Multiaddr>>> {
        let (out, inlet) = oneshot::channel();
        let cmd = Command::LocalLookup { peer, out };
        let send = self.outlet.send(cmd);
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

    fn neighborhood(&self, key: Multihash<64>, count: usize) -> Future<Result<Vec<PeerId>>> {
        self.execute(|out| Command::Neighborhood { key, count, out })
    }
}
