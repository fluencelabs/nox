/*
 * Copyright 2018 Fluence Labs Limited
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

use contract_status::cluster::{Cluster, ClusterMember};
use contract_status::code::Code;
use contract_status::node::Node;
use std::boxed::Box;
use std::error::Error;
use std::fmt;
use types::NodeAddress;
use utils;
use web3::types::{Address, H256, U256};

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    clusters: Vec<Cluster>,
    enqueued_codes: Vec<Code>,
    ready_nodes: Vec<Node>,
}

impl Status {
    pub fn new(
        clusters: Vec<Cluster>,
        enqueued_codes: Vec<Code>,
        ready_nodes: Vec<Node>,
    ) -> Status {
        Status {
            clusters,
            enqueued_codes,
            ready_nodes,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Status: (\n\tclusters: {:?},\n\tready nodes:{:?},\n\tenqueued codes lengths: {:?}\n)",
            self.clusters, self.ready_nodes, self.enqueued_codes
        )
    }
}

pub fn get_enqueued_codes(
    contract_address: Address,
    eth_url: &str,
) -> Result<Vec<Code>, Box<Error>> {
    let options = utils::options();

    let (storage_hashes, storage_receipts, cluster_sizes): (Vec<H256>, Vec<H256>, Vec<u64>) =
        utils::query_contract(contract_address, eth_url, "getEnqueuedCodes", (), options)?;

    let mut codes: Vec<Code> = Vec::new();
    for i in 0..storage_hashes.len() {
        let code = Code::new(
            storage_hashes[i],
            storage_receipts[i],
            cluster_sizes[i] as u8,
        );
        codes.push(code);
    }

    Ok(codes)
}

pub fn get_ready_nodes(contract_address: Address, eth_url: &str) -> Result<Vec<Node>, Box<Error>> {
    let options = utils::options();

    let (nodes_indices, node_addresses, start_ports, end_ports, current_ports): (
        Vec<H256>,
        Vec<NodeAddress>,
        Vec<u64>,
        Vec<u64>,
        Vec<u64>,
    ) = utils::query_contract(contract_address, eth_url, "getReadyNodes", (), options)?;

    let mut nodes: Vec<Node> = Vec::new();
    for i in 0..nodes_indices.len() {
        let node = Node::new(
            nodes_indices[i],
            node_addresses[i],
            start_ports[i] as u16,
            end_ports[i] as u16,
            current_ports[i] as u16,
        );
        nodes.push(node);
    }

    Ok(nodes)
}

pub fn get_clusters(contract_address: Address, eth_url: &str) -> Result<Vec<Cluster>, Box<Error>> {
    let options = utils::options();

    let (cluster_ids, genesis_times, storage_hashes, storage_receipts, cluster_sizes): (
        Vec<H256>,
        Vec<U256>,
        Vec<H256>,
        Vec<H256>,
        Vec<u64>,
    ) = utils::query_contract(contract_address, eth_url, "getClustersInfo", (), options)?;

    let options2 = utils::options();

    let (nodes_ids, nodes_addresses, ports): (Vec<H256>, Vec<NodeAddress>, Vec<u64>) =
        utils::query_contract(contract_address, eth_url, "getClustersNodes", (), options2)?;

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut nodes_counter = 0;

    for i in 0..storage_hashes.len() {
        let cluster_size = cluster_sizes[i];

        let mut cluster_members: Vec<ClusterMember> = Vec::new();

        for _j in 0..cluster_size {
            let id = nodes_ids[nodes_counter];
            let address = nodes_addresses[nodes_counter];
            let port = ports[nodes_counter] as u16;

            cluster_members.push(ClusterMember::new(id, address, port));

            nodes_counter = nodes_counter + 1;
        }

        let code = Code::new(
            storage_hashes[i],
            storage_receipts[i],
            cluster_sizes[i] as u8,
        );

        let cluster = Cluster::new(cluster_ids[i], genesis_times[i], code, cluster_members);

        clusters.push(cluster);
    }

    Ok(clusters)
}
