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

use libp2p::core::Multiaddr;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_CERT_DIR: &str = ".fluence/certificates";
pub const DEFAULT_KEY_DIR: &str = ".fluence/secret_key";
pub const DEFAULT_CONFIG_FILE: &str = ".fluence/Config.toml";
pub const DEFAULT_SERVICES_BASE_DIR: &str = ".fluence/services";
pub const DEFAULT_STEPPER_BASE_DIR: &str = ".fluence/stepper";

pub fn default_tcp_port() -> u16 {
    7777
}
pub fn default_listen_ip() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}
pub fn default_socket_timeout() -> Duration {
    Duration::from_secs(20)
}
pub fn default_bootstrap_nodes() -> Vec<Multiaddr> {
    vec![]
}
pub fn default_websocket_port() -> u16 {
    9999
}
pub fn default_prometheus_port() -> u16 {
    18080
}
pub fn default_cert_dir() -> String {
    DEFAULT_CERT_DIR.into()
}

pub fn default_services_basedir() -> PathBuf {
    DEFAULT_SERVICES_BASE_DIR.into()
}

pub fn default_stepper_basedir() -> PathBuf {
    DEFAULT_STEPPER_BASE_DIR.into()
}

pub fn default_air_interpreter_path() -> PathBuf {
    use air_interpreter_wasm as interpreter;

    PathBuf::from(format!("./aquamarine_{}.wasm", interpreter::VERSION))
}

pub fn default_stepper_pool_size() -> usize {
    num_cpus::get() * 2
}

pub fn default_particle_queue_buffer_size() -> usize {
    100
}
