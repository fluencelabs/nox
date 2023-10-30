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
mod node;
mod tasks;
mod metrics;

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
pub use layers::log_layer;
pub use layers::tokio_console_layer;
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
