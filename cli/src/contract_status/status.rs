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
use contract_status::cluster::{get_clusters, Cluster};
use contract_status::app::{get_enqueued_apps, App};
use contract_status::node::{get_ready_nodes, Node};
use std::boxed::Box;
use std::error::Error;
use web3::types::Address;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Status {
    clusters: Vec<Cluster>,
    enqueued_codes: Vec<App>,
    ready_nodes: Vec<Node>,
}

impl Status {
    pub fn new(
        clusters: Vec<Cluster>,
        enqueued_codes: Vec<App>,
        ready_nodes: Vec<Node>,
    ) -> Status {
        Status {
            clusters,
            enqueued_codes,
            ready_nodes,
        }
    }
}

/// Gets status about Fluence contract from ethereum blockchain.
pub fn get_status(contract_address: Address, eth_url: &str) -> Result<Status, Box<Error>> {
    let contract = ContractCaller::new(contract_address, eth_url)?;

    let nodes = get_ready_nodes(&contract)?;

    let clusters = get_clusters(&contract)?;
    let codes = get_enqueued_apps(&contract)?;

    Ok(Status::new(clusters, codes, nodes))
}
