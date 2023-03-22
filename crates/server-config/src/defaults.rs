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

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fluence_keypair::KeyPair;
use libp2p::core::{Multiaddr};
use libp2p::identity::ed25519::Keypair;
use libp2p::identity::PublicKey;
use libp2p::PeerId;

use fluence_libp2p::Transport;

use crate::node_config::{KeypairConfig, PathOrValue};

const CONFIG_VERSION: usize = 1;

pub fn default_transport() -> Transport {
    Transport::Network
}

pub fn default_config_path() -> PathBuf {
    default_base_dir().join("Config.toml")
}

pub fn default_tcp_port() -> u16 {
    7777
}

pub fn default_listen_ip() -> IpAddr {
    "0.0.0.0".parse().unwrap()
}

pub fn default_socket_timeout() -> Duration {
    Duration::from_secs(20)
}

pub fn default_max_established_per_peer_limit() -> Option<u32> {
    Some(5)
}

pub fn default_auto_particle_ttl() -> Duration {
    Duration::from_secs(200)
}

pub fn default_bootstrap_nodes() -> Vec<Multiaddr> {
    vec![]
}

pub fn default_websocket_port() -> u16 {
    9999
}

pub fn default_metrics_port() -> u16 {
    18080
}

pub fn default_metrics_enabled() -> bool {
    true
}

pub fn default_services_metrics_timer_resolution() -> Duration {
    Duration::from_secs(60)
}

pub fn default_base_dir() -> PathBuf {
    format!(".fluence/v{CONFIG_VERSION}").into()
}

pub fn services_base_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("services")
}

pub fn builtins_base_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("builtins")
}

pub fn avm_base_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("stepper")
}

pub fn default_keypair_path() -> PathOrValue {
    PathOrValue::Path {
        path: default_base_dir().join("secret_key.ed25519"),
    }
}

pub fn default_builtins_keypair_path() -> PathOrValue {
    PathOrValue::Path {
        path: default_base_dir().join("builtins_secret_key.ed25519"),
    }
}

pub fn default_root_keypair() -> KeyPair {
    let config = KeypairConfig {
        format: default_key_format(),
        keypair: None,
        secret_key: None,
        generate_on_absence: true,
    };

    config
        // TODO: respect base_dir https://github.com/fluencelabs/fluence/issues/1210
        .get_keypair(default_keypair_path())
        .expect("generate default root keypair")
}

pub fn default_builtins_keypair() -> KeyPair {
    let config = KeypairConfig {
        format: default_key_format(),
        keypair: None,
        secret_key: None,
        generate_on_absence: true,
    };

    config
        // TODO: respect base_dir https://github.com/fluencelabs/fluence/issues/1210
        .get_keypair(default_builtins_keypair_path())
        .expect("generate default builtins keypair")
}

pub fn default_aquavm_pool_size() -> usize {
    num_cpus::get() * 2
}

pub fn default_particle_queue_buffer_size() -> usize {
    100
}

pub fn default_particle_processor_parallelism() -> Option<usize> {
    Some(num_cpus::get() * 2)
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

pub fn default_max_spell_particle_ttl() -> Duration {
    Duration::from_secs(120)
}

pub fn default_bootstrap_frequency() -> usize {
    3
}

pub fn default_execution_timeout() -> Duration {
    Duration::from_secs(20)
}

pub fn default_autodeploy_retry_attempts() -> u16 {
    5
}

pub fn default_processing_timeout() -> Duration {
    Duration::from_secs(120)
}

pub fn default_management_peer_id() -> PeerId {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};

    let kp = Keypair::generate();
    let public_key = PublicKey::Ed25519(kp.public());
    let peer_id = PeerId::from(public_key);

    log::warn!(
        "New management key generated. ed25519 private key in base64 = {}",
        base64.encode(kp.secret()),
    );
    peer_id
}

pub fn default_key_format() -> String {
    "ed25519".to_string()
}

pub fn default_module_max_heap_size() -> bytesize::ByteSize {
    bytesize::ByteSize::b(bytesize::gib(4_u64) - 1)
}

pub fn default_max_builtin_metrics_storage_size() -> usize {
    5
}
