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

use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::info::Info;
use prometheus_client::registry::Registry;

pub struct NoxInfo  {
    pub versions: NoxVersions,
    pub chain_info: ChainInfo,
}


#[derive(Debug, Clone, Hash, Eq, PartialEq, EncodeLabelSet)]
pub struct NoxVersions {
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
    pub fn empty(peer_id: String) -> ChainInfo {
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

pub fn add_info_metrics(
    registry: &mut Registry,
    nox_info: NoxInfo,
) {
    let sub_registry = registry.sub_registry_with_prefix("nox");

    let info = Info::new(nox_info.versions);
    sub_registry.register("build", "Nox Info", info);

    let chain_info = Info::new(nox_info.chain_info);
    sub_registry.register("chain", "Chain Nox Info", chain_info);
}

pub fn add_info_metrics2(
    registry: &mut Registry,
    node_version: String,
    air_version: String,
    spell_version: String,
) {
    let sub_registry = registry.sub_registry_with_prefix("nox");

    let info = Info::new(vec![
        ("node_version", node_version),
        ("air_version", air_version),
        ("spell_version", spell_version),
    ]);
    sub_registry.register("build", "Nox Info", info);
}