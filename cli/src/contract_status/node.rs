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
#![allow(dead_code)]

use crate::contract_func::ContractCaller;
use crate::types::NodeAddress;
use std::error::Error;
use web3::types::{Address, H256};

/// Represents Fluence node registered in ethereum contract.
/// The node listens to contract events and runs real-time nodes.
/// The purpose of real-time nodes is to host developer's [`App`], e.g., backend code.
#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Node {
    id: H256,
    tendermint_key: String,
    ip_addr: String,
    next_port: u16,
    last_port: u16,
    owner: Address,
    is_private: bool,
    clusters_ids: Option<Vec<H256>>, // Defined if loaded
}

impl Node {
    pub fn new(
        id: H256,
        address: NodeAddress,
        next_port: u16,
        last_port: u16,
        owner: Address,
        is_private: bool,
        clusters_ids: Option<Vec<H256>>,
    ) -> Result<Node, Box<Error>> {
        let (tendermint_key, ip_addr) = address.decode()?;
        Ok(Node {
            id,
            tendermint_key,
            ip_addr,
            next_port,
            last_port,
            owner,
            is_private,
            clusters_ids,
        })
    }
}

/// Gets list of nodes from Fluence contract
// TODO: implement
pub fn get_nodes(_contract: &ContractCaller) -> Result<Vec<Node>, Box<Error>> {
    let nodes: Vec<Node> = Vec::new();
    Ok(nodes)
}
