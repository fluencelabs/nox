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

use contract_status::cluster::{Cluster, ClusterMember, get_clusters};
use contract_status::code::{Code, get_enqueued_codes};
use contract_status::node::{Node, get_ready_nodes};
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
    pub fn clusters(&self) -> &Vec<Cluster> {
        &self.clusters
    }

    pub fn enqueued_codes(&self) -> &Vec<Code> {
        &self.enqueued_codes
    }

    pub fn ready_nodes(&self) -> &Vec<Node> {
        &self.ready_nodes
    }

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

/// Gets status about Fluence contract from ethereum blockchain.
pub fn get_status(contract_address: Address, eth_url: &str) -> Result<Status, Box<Error>> {
    let nodes = get_ready_nodes(contract_address, eth_url)?;

    let clusters = get_clusters(contract_address, eth_url)?;
    let codes = get_enqueued_codes(contract_address, eth_url)?;

    Ok(Status::new(clusters, codes, nodes))
}
