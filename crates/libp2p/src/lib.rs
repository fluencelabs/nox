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

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod connected_point;
mod macros;
pub mod random_multiaddr;
mod random_peer_id;
mod serde;
#[cfg(feature = "tokio")]
mod transport;

pub use self::serde::*;
pub use connected_point::*;
pub use random_peer_id::RandomPeerId;
#[cfg(feature = "tokio")]
pub use transport::{build_memory_transport, build_transport, Transport};

// libp2p reexports
pub use libp2p::PeerId;
