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

use fluence_libp2p::peerid_serializer;
use fluence_libp2p::types::{OneshotInlet, OneshotOutlet, Outlet};
use particle_protocol::Particle;

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use itertools::Itertools;
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use serde::export::Formatter;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Contact {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
}

impl Contact {
    pub fn new(peer_id: PeerId, addresses: Vec<Multiaddr>) -> Self {
        Self { peer_id, addresses }
    }
}

impl Display for Contact {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.addresses.is_empty() {
            write!(f, "{} @ {}", self.peer_id, "[no addr]")
        } else {
            let addrs = self.addresses.iter().join(" ");
            write!(f, "{} @ [{}]", self.peer_id, addrs)
        }
    }
}

#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    Connected(Contact),
    Disconnected(Contact),
}

pub trait ConnectionPoolT {
    fn dial(&self, addr: Multiaddr) -> BoxFuture<'static, Option<Contact>>;
    fn connect(&self, contact: Contact) -> BoxFuture<'static, bool>;
    fn disconnect(&self, contact: Contact) -> BoxFuture<'static, bool>;
    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool>;
    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>>;
    fn send(&self, to: Contact, particle: Particle) -> BoxFuture<'static, bool>;
    fn count_connections(&self) -> BoxFuture<'static, usize>;
    fn lifecycle_events(&self) -> BoxStream<'static, LifecycleEvent>;
}
