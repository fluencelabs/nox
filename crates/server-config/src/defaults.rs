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
use libp2p::identity::ed25519::Keypair;
use libp2p::PeerId;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CONFIG_VERSION: usize = 1;

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

pub fn default_base_dir() -> PathBuf {
    format!(".fluence/v{}", CONFIG_VERSION).into()
}

pub fn cert_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("certificates")
}

pub fn services_basedir(base_dir: &Path) -> PathBuf {
    base_dir.join("services")
}

pub fn stepper_basedir(base_dir: &Path) -> PathBuf {
    base_dir.join("stepper")
}

pub fn default_config_file() -> PathBuf {
    default_base_dir().join("Config.toml")
}

pub fn air_interpreter_path(base_dir: &Path) -> PathBuf {
    use air_interpreter_wasm as interpreter;

    base_dir.join(format!("aquamarine_{}.wasm", interpreter::VERSION))
}

pub fn default_stepper_pool_size() -> usize {
    num_cpus::get() * 2
}

pub fn default_particle_queue_buffer_size() -> usize {
    100
}

pub fn default_particle_processor_parallelism() -> usize {
    32
}

pub fn default_script_storage_timer_resolution() -> Duration {
    Duration::from_secs(3)
}

pub fn default_script_storage_max_failures() -> u8 {
    3
}

pub fn default_script_storage_particle_ttl() -> Duration {
    Duration::from_secs(120)
}

pub fn default_bootstrap_frequency() -> usize {
    3
}

pub fn default_execution_timeout() -> Duration {
    Duration::from_secs(20)
}

pub fn default_processing_timeout() -> Duration {
    Duration::from_secs(120)
}

pub fn default_management_peer_id() -> PeerId {
    let kp = Keypair::generate();
    let secret = kp.secret();
    let secret = base64::encode(secret);
    let public_key = libp2p::identity::PublicKey::Ed25519(kp.public());

    let peer_id = PeerId::from(public_key);
    log::warn!(
        "New management key generated. private in base64 = {}; peer_id = {}",
        secret,
        peer_id.to_base58()
    );
    peer_id
}
