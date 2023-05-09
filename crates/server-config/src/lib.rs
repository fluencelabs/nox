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
#![feature(try_trait_v2)]
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

pub mod args;
mod bootstrap_config;
mod defaults;
mod dir_config;
mod kademlia_config;
mod keys;
mod network_config;
mod node_config;
mod resolved_config;
mod services_config;

pub use defaults::{builtins_base_dir, *};
pub use resolved_config::load_config;
pub use resolved_config::load_config_with_args;
pub use resolved_config::ConfigData;

pub use bootstrap_config::BootstrapConfig;
pub use kademlia_config::KademliaConfig;
pub use network_config::NetworkConfig;
pub use node_config::{NodeConfig, TransportConfig};
pub use resolved_config::LogConfig;
pub use resolved_config::LogFormat;
pub use resolved_config::{ResolvedConfig, UnresolvedConfig};
pub use services_config::ServicesConfig;
