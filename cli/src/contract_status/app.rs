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

use std::error::Error;

use derive_getters::Getters;
use serde_derive::{Deserialize, Serialize};
use web3::types::{Address, H256};

use crate::contract_func::contract::functions::get_app;
use crate::contract_func::contract::functions::get_nodes_ids;
use crate::contract_func::contract::functions::get_node;
use crate::contract_func::contract::functions::get_app_i_ds;
use crate::contract_func::ContractCaller;
use crate::contract_status::node::Node;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct App {
    app_id: H256,
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
    owner: Address,
    pin_to_nodes: Option<Vec<H256>>,
    cluster: Option<Cluster>,
}

impl App {
    pub fn new(
        app_id: H256,
        storage_hash: H256,
        storage_receipt: H256,
        cluster_size: u8,
        owner: Address,
        pin_to_nodes: Option<Vec<H256>>,
        cluster: Option<Cluster>,
    ) -> App {
        App {
            app_id,
            storage_hash,
            storage_receipt,
            cluster_size,
            owner,
            pin_to_nodes,
            cluster,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Cluster {
    genesis_time: u32,
    node_ids: Vec<H256>,
    ports: Vec<u16>,
}

impl Cluster {
    pub fn new(genesis_time: u32, node_ids: Vec<H256>, ports: Vec<u16>) -> Cluster {
        Cluster {
            genesis_time,
            node_ids,
            ports,
        }
    }
}

pub fn get_nodes(contract: &ContractCaller) -> Result<Vec<Node>, Box<Error>> {
    let (call_data, decoder) = get_nodes_ids::call();
    let node_ids: Vec<H256> = contract.query_contract(call_data, Box::new(decoder))?;

    let nodes: Result<Vec<Node>, Box<Error>> = node_ids
        .iter()
        .map(|id| {
            let (call_data, decoder) = get_node::call(*id);
            let (
                ip_addr,
                next_port,
                last_port,
                owner,
                is_private,
                app_ids
            ) = contract.query_contract(call_data, Box::new(decoder))?;

            Node::new (
                *id,
                ip_addr.into(),
                Into::<u64>::into(next_port) as u16,
                Into::<u64>::into(last_port) as u16,
                owner,
                is_private,
                Some(app_ids)
            )
        })
        .collect();

    Ok(nodes?)
}

pub fn get_apps(contract: &ContractCaller) -> Result<Vec<App>, Box<Error>> {
    let (call_data, decoder) = get_app_i_ds::call();
    let app_ids: Vec<H256> = contract.query_contract(call_data, Box::new(decoder))?;

    let apps: Result<Vec<App>, Box<Error>> = app_ids
        .iter()
        .map(|id| {
            let (call_data, decoder) = get_app::call(*id);
            let (
                storage_hash,
                storage_receipt,
                cluster_size,
                owner,
                pin_to,
                genesis,
                node_ids,
                ports,
            ) = contract.query_contract(call_data, Box::new(decoder))?;

            let cluster = if !genesis.is_zero() {
                let genesis: u64 = genesis.into();
                let ports = ports
                    .iter()
                    .map(|p| (Into::<u64>::into(*p) as u16))
                    .collect();

                Some(Cluster::new(genesis as u32, node_ids, ports))
            } else {
                None
            };

            let cluster_size: u64 = cluster_size.into();

            let app = App::new(
                *id,
                storage_hash,
                storage_receipt,
                cluster_size as u8,
                owner,
                Some(pin_to),
                cluster,
            );

            Ok(app)
        })
        .collect();

    Ok(apps?)
}
