/*
 * Copyright 2024 Fluence DAO
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
