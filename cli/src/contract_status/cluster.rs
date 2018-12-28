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

use contract_func::ContractCaller;
use contract_status::app::App;
use std::error::Error;
use types::NodeAddress;
use web3::types::{Address, H256, U256};

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Worker {
    node_id: H256,
    port: u16,
}

impl Worker {
    pub fn new(
        node_id: H256,
        port: u16,
    ) -> Result<Worker, Box<Error>> {
        Ok(Worker {
            node_id,
            port,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Cluster {
    id: H256,
    app: App,
    genesis_time: U256,
    workers: Vec<Worker>,
}

impl Cluster {
    pub fn new(
        id: H256,
        app: App,
        genesis_time: U256,
        workers: Vec<Worker>,
    ) -> Cluster {
        Cluster {
            id,
            genesis_time,
            app,
            workers,
        }
    }
}

/// Gets list of formed clusters from Fluence contract
pub fn get_clusters(contract: &ContractCaller) -> Result<Vec<Cluster>, Box<Error>> {
    let (cluster_ids, genesis_times, code_addresses, storage_receipts, cluster_sizes, owners): (
        Vec<H256>,
        Vec<U256>,
        Vec<H256>,
        Vec<H256>,
        Vec<u64>,
        Vec<Address>,
    ) = contract.query_contract(
        "getClustersInfo",
        (),
    )?;

    let (nodes_ids, nodes_addresses, ports, owners): (
        Vec<H256>,
        Vec<NodeAddress>,
        Vec<u64>,
        Vec<Address>,
    ) = contract.query_contract("getClustersNodes", ())?;

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut nodes_counter = 0;

    for i in 0..code_addresses.len() {
        let cluster_size = cluster_sizes[i];

        let mut cluster_members: Vec<Worker> = Vec::new();

        for _j in 0..cluster_size {
            let id = nodes_ids[nodes_counter];
            let address = nodes_addresses[nodes_counter];
            let port = ports[nodes_counter] as u16;
            let owner = owners[nodes_counter];

            let cluster_member = Worker::new(id, address, port, owner)?;

            cluster_members.push(cluster_member);

            nodes_counter = nodes_counter + 1;
        }

        let app = App::new(
            code_addresses[i],
            storage_receipts[i],
            cluster_sizes[i] as u8,
            owners[i],
        );

        let cluster = Cluster::new(cluster_ids[i], genesis_times[i], app, cluster_members);

        clusters.push(cluster);
    }

    Ok(clusters)
}
