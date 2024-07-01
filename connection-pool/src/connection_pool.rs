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

use std::fmt::{Display, Formatter};

use futures::{future::BoxFuture, stream::BoxStream};
use libp2p::{core::Multiaddr, PeerId};

use particle_protocol::{Contact, ExtendedParticle, SendStatus};

#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    Connected(Contact),
    Disconnected(Contact),
}

impl Display for LifecycleEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleEvent::Connected(c) => write!(f, "Connected {c}"),
            LifecycleEvent::Disconnected(c) => write!(f, "Disconnected {c}"),
        }
    }
}

pub trait ConnectionPoolT {
    fn dial(&self, addr: Multiaddr) -> BoxFuture<'static, Option<Contact>>;
    fn connect(&self, contact: Contact) -> BoxFuture<'static, bool>;
    fn disconnect(&self, peer_id: PeerId) -> BoxFuture<'static, bool>;
    fn is_connected(&self, peer_id: PeerId) -> BoxFuture<'static, bool>;
    fn get_contact(&self, peer_id: PeerId) -> BoxFuture<'static, Option<Contact>>;
    fn send(&self, to: Contact, particle: ExtendedParticle) -> BoxFuture<'static, SendStatus>;
    fn count_connections(&self) -> BoxFuture<'static, usize>;
    fn lifecycle_events(&self) -> BoxStream<'static, LifecycleEvent>;
}
