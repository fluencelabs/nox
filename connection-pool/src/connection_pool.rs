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
use libp2p::core::Multiaddr;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Contact {
    #[serde(with = "peerid_serializer")]
    pub peer_id: PeerId,
    pub addr: Option<Multiaddr>,
}

pub trait ConnectionPoolT {
    fn connect(&self, contact: Contact) -> BoxFuture<'static, bool>;
    fn disconnect(&self, contact: Contact) -> BoxFuture<'static, bool>;
    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool>;
    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>>;
    fn send(&self, to: Contact, particle: Particle) -> BoxFuture<'static, bool>;
}
