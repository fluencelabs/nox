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
#![feature(extend_one)]
#![feature(try_blocks)]
#![feature(ip)]
#![feature(extract_if)]
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

mod builtins;
mod connectivity;
mod dispatcher;
mod effectors;
mod health;
mod http;
mod layers;
mod metrics;
mod node;
mod tasks;
mod behaviour {
    mod identify;
    mod network;

    pub use network::{FluenceNetworkBehaviour, FluenceNetworkBehaviourEvent};
}

pub use behaviour::{FluenceNetworkBehaviour, FluenceNetworkBehaviourEvent};
pub use http::StartedHttp;
pub use node::Node;

// to be available in benchmarks
pub use connection_pool::Command as ConnectionPoolCommand;
pub use connectivity::Connectivity;
pub use kademlia::Command as KademliaCommand;
pub use layers::env_filter;
pub use layers::log_layer;
pub use layers::tracing_layer;

#[derive(Debug, Clone)]
pub struct Versions {
    pub node_version: String,
    pub avm_version: String,
    pub spell_version: String,
    pub system_service: system_services::Versions,
}

impl Versions {
    pub fn new(
        node_version: String,
        avm_version: String,
        spell_version: String,
        system_service: system_services::Versions,
    ) -> Self {
        Self {
            node_version,
            avm_version,
            spell_version,
            system_service,
        }
    }
}
