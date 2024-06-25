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
#![feature(async_closure)]
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

mod libp2p_protocol {
    mod codec;
    pub(super) mod message;
    pub(super) mod upgrade;
}

mod contact;
mod error;
mod particle;

pub use contact::Contact;
pub use error::ParticleError;
pub use libp2p_protocol::message::CompletionChannel;
pub use libp2p_protocol::message::SendStatus;
pub use libp2p_protocol::message::{HandlerMessage, ProtocolMessage};
pub use libp2p_protocol::upgrade::ProtocolConfig;
pub use particle::ExtendedParticle;
pub use particle::Particle;

pub const PROTOCOL_NAME: &str = "/fluence/particle/2.0.0";
