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
#![feature(try_blocks)]
#![feature(drain_filter)]
#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![allow(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod connectivity;
mod dispatcher;
mod effectors;
mod metrics;
mod node;
mod tasks;

mod behaviour {
    mod identify;
    mod network;

    pub use network::NetworkBehaviour;
}

pub mod config {
    mod args;

    pub mod certificates;

    pub use args::create_args;
}

pub use behaviour::NetworkBehaviour;
pub use node::Node;

// to be available in benchmarks
pub use connection_pool::Command as ConnectionPoolCommand;
pub use connectivity::Connectivity;
pub use kademlia::Command as KademliaCommand;
