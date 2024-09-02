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
mod avm_config;
mod bootstrap_config;
mod defaults;
mod dir_config;
mod kademlia_config;
mod keys;
mod network_config;
mod node_config;
mod resolved_config;
mod services_config;
pub mod system_services_config;
mod wasm_backend_config;

pub use defaults::*;
pub use resolved_config::load_config;
pub use resolved_config::load_config_with_args;
pub use resolved_config::ConfigData;

pub use bootstrap_config::BootstrapConfig;
pub use kademlia_config::KademliaConfig;
pub use network_config::NetworkConfig;
pub use node_config::{
    ChainConfig, ChainListenerConfig, Network, NodeConfig, TransportConfig, VmNetworkConfig,
};
pub use resolved_config::TracingConfig;
pub use resolved_config::{ResolvedConfig, UnresolvedConfig};
pub use system_services_config::{AquaIpfsConfig, DeciderConfig, SystemServicesConfig};
