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

use libp2p::core::Multiaddr;
use libp2p::identity::ed25519::Keypair;
use libp2p::identity::PublicKey;
use libp2p::PeerId;

use fluence_libp2p::Transport;

use crate::node_config::PathOrValue;
use crate::system_services_config::ServiceKey;

const CONFIG_VERSION: usize = 1;

pub fn default_transport() -> Transport {
    Transport::Network
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

pub fn default_connection_idle_timeout() -> Duration {
    Duration::from_secs(10)
}

pub fn default_max_established_per_peer_limit() -> Option<u32> {
    Some(5)
}

pub fn default_bootstrap_nodes() -> Vec<Multiaddr> {
    vec![]
}

pub fn default_system_cpu_count() -> usize {
    let total = num_cpus::get_physical();
    match total {
        x if x > 32 => 4,
        x if x > 2 => 2,
        _ => 1,
    }
}

pub fn default_websocket_port() -> u16 {
    9999
}

pub fn default_http_port() -> u16 {
    18080
}

pub fn default_metrics_enabled() -> bool {
    true
}

pub fn default_tokio_metrics_enabled() -> bool {
    false
}

pub fn default_tokio_metrics_poll_histogram_enabled() -> bool {
    false
}

pub fn default_health_check_enabled() -> bool {
    true
}

pub fn default_services_metrics_timer_resolution() -> Duration {
    Duration::from_secs(60)
}

pub fn default_base_dir() -> PathBuf {
    format!(".fluence/v{CONFIG_VERSION}").into()
}

pub fn persistent_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("persistent")
}
pub fn ephemeral_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("ephemeral")
}

pub fn services_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("services")
}

pub fn avm_base_dir(base_dir: &Path) -> PathBuf {
    base_dir.join("stepper")
}

pub fn default_keypair_path(base_dir: &Path) -> PathOrValue {
    PathOrValue::Path {
        path: base_dir.join("secret_key.ed25519"),
    }
}

pub fn default_builtins_keypair_path(persistent_base_dir: &Path) -> PathOrValue {
    PathOrValue::Path {
        path: persistent_base_dir.join("builtins_secret_key.ed25519"),
    }
}

pub fn default_aquavm_pool_size() -> usize {
    num_cpus::get() * 2
}

pub fn default_particle_queue_buffer_size() -> usize {
    128
}

pub fn default_effects_queue_buffer_size() -> usize {
    128
}

pub fn default_particle_processor_parallelism() -> Option<usize> {
    Some(num_cpus::get() * 2)
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

pub fn default_processing_timeout() -> Duration {
    Duration::from_secs(120)
}

pub fn default_management_peer_id() -> PeerId {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};

    let kp = Keypair::generate();
    let public_key: PublicKey = PublicKey::from(kp.public()); //TODO: safe unwrap
    let peer_id = PeerId::from(public_key);

    log::info!(
        "New management key generated. ed25519 private key in base64 = {}",
        base64.encode(kp.secret()),
    );
    peer_id
}

pub fn default_key_format() -> String {
    "ed25519".to_string()
}

pub fn default_service_memory_limit() -> bytesize::ByteSize {
    bytesize::ByteSize::b(bytesize::gib(4_u64) - 1)
}

pub fn default_max_builtin_metrics_storage_size() -> usize {
    5
}

pub fn default_allowed_binaries() -> Vec<String> {
    vec!["/usr/bin/curl".to_string(), "/usr/bin/ipfs".to_string()]
}

pub fn default_system_services() -> Vec<ServiceKey> {
    ServiceKey::all_values()
}

pub fn default_ipfs_multiaddr() -> String {
    "/dns4/ipfs.fluence.dev/tcp/5001".to_string()
}

// 15 minutes
pub fn default_worker_spell_period_sec() -> u32 {
    900
}

// 2 minutes
pub fn default_decider_spell_period_sec() -> u32 {
    120
}

// 60 minutes
// This is an interval setting for a spell in general.
// should be the smallest common denominator of other intervals.
pub fn default_registry_spell_period_sec() -> u32 {
    3600
}

// 24 hours
pub fn default_registry_expired_spell_period_sec() -> u32 {
    86400
}

// 12 hours
pub fn default_registry_renew_spell_period_sec() -> u32 {
    43200
}

// 60 minutes
pub fn default_registry_replicate_spell_period_sec() -> u32 {
    3600
}

pub fn default_decider_network_api_endpoint() -> String {
    "https://endpoints.omniatech.io/v1/matic/mumbai/public".to_string()
}

pub fn default_matcher_address() -> String {
    // on mumbai
    "0x93A2897deDcC5478a9581808F5EC25F4FadbC312".to_string()
}

pub fn default_decider_start_block_hex() -> String {
    "latest".to_string()
}

pub fn default_decider_worker_gas() -> u64 {
    210_000
}

pub fn default_ipfs_binary_path() -> String {
    "/usr/bin/ipfs".to_string()
}

pub fn default_curl_binary_path() -> String {
    "/usr/bin/curl".to_string()
}

pub fn default_decider_network_id() -> u64 {
    // 80001 = polygon mumbai
    80001
}
