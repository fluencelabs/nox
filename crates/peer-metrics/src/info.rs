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

use std::fmt::{Error, Write};

use prometheus_client::encoding::{EncodeLabelSet, EncodeLabelValue, LabelValueEncoder};
use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

pub struct NoxInfo {
    pub version: NoxVersion,
    pub chain_info: ChainInfo,
    pub vm_info: VmInfo,
    pub network_info: NetworkInfo,
    pub system_info: SystemInfo,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct NoxVersion {
    pub node_version: String,
    pub air_version: String,
    pub spell_version: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct ChainInfo {
    pub peer_id: String,
    // Connector Settings
    pub http_endpoint: String,
    pub diamond_contract_address: String,
    pub network_id: u64,
    pub default_base_fee: Option<u64>,
    pub default_priority_fee: Option<u64>,

    // Listener Settings
    pub ws_endpoint: String,
    pub proof_poll_period_secs: u64,
    pub min_batch_count: usize,
    pub max_batch_count: usize,
    pub max_proof_batch_size: usize,
    pub epoch_end_window_secs: u64,
}

impl ChainInfo {
    pub fn default(peer_id: String) -> ChainInfo {
        ChainInfo {
            peer_id,
            http_endpoint: "".to_string(),
            diamond_contract_address: "".to_string(),
            network_id: 0,
            default_base_fee: None,
            default_priority_fee: None,
            ws_endpoint: "".to_string(),
            proof_poll_period_secs: 0,
            min_batch_count: 0,
            max_batch_count: 0,
            max_proof_batch_size: 0,
            epoch_end_window_secs: 0,
        }
    }
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct SystemInfo {
    pub cpus_range: String,
    pub system_cpu_count: usize,
    pub particle_execution_timeout_sec: u64,
    pub max_spell_particle_ttl_sec: u64,
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct VmInfo {
    pub allow_gpu: u8,
    pub public_ip: String,
    pub host_ssh_port: u16,
    pub vm_ssh_port: u16,
    pub port_range_start: u16,
    pub port_range_end: u16,
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct NetworkInfo {
    pub tcp_port: u16,
    pub websocket_port: u16,
    pub listen_ip: String,
    pub network_type: String,
    pub bootstrap_nodes: Addresses,
    pub external_address: Option<String>,
    pub external_multiaddresses: Addresses,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Default)]
pub struct Addresses(pub Vec<String>);
impl EncodeLabelValue for Addresses {
    fn encode(&self, encoder: &mut LabelValueEncoder) -> Result<(), Error> {
        encoder.write_str(&self.0.join(", "))
    }
}

pub fn add_info_metrics(registry: &mut Registry, nox_info: NoxInfo) {
    let sub_registry = registry.sub_registry_with_prefix("nox");

    let info = Info::new(nox_info.version);
    sub_registry.register("build", "Nox Info", info);

    let chain_info = Info::new(nox_info.chain_info);
    sub_registry.register("chain", "Chain Nox Info", chain_info);

    let network_info = Info::new(nox_info.network_info);
    sub_registry.register("network", "Network Nox Info", network_info);

    let vm_info = Info::new(nox_info.vm_info);
    sub_registry.register("vm", "VM Nox Info", vm_info);

    let system_info = Info::new(nox_info.system_info);
    sub_registry.register("system", "System Nox Info", system_info);
}
