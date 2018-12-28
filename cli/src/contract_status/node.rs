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
use std::error::Error;
use types::NodeAddress;
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
pub fn get_nodes(contract: &ContractCaller) -> Result<Vec<Node>, Box<Error>> {
    let (ids, addresses, next_ports, last_ports, owners, is_private): (
        Vec<H256>,
        Vec<NodeAddress>,
        Vec<u64>,
        Vec<u64>,
        Vec<Address>,
        Vec<Bool>,
    ) = contract.query_contract("getNodes", ())?;

    let mut nodes: Vec<Node> = Vec::new();
    for i in 0..nodes_indices.len() {
        let node = Node::new(
            ids[i],
            addresses[i],
            next_ports[i] as u16,
            last_ports[i] as u16,
            owners[i],
            is_private[i] as Bool,
            None,
        )?;
        nodes.push(node);
    }

    Ok(nodes)
}
