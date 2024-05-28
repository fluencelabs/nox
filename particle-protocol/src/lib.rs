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
