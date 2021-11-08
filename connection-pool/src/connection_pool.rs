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

use particle_protocol::{Contact, Particle};

use futures::{future::BoxFuture, stream::BoxStream};
use libp2p::{core::Multiaddr, PeerId};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    Connected(Contact),
    Disconnected(Contact),
}

impl Display for LifecycleEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleEvent::Connected(c) => write!(f, "Connected {}", c),
            LifecycleEvent::Disconnected(c) => write!(f, "Disconnected {}", c),
        }
    }
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
