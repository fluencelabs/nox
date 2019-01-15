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

use web3::types::{Address, H256};

use contract_func::contract::functions::get_app_i_ds;
use contract_func::contract::functions::get_cluster;
use contract_func::contract::functions::get_enqueued_apps;
use contract_func::ContractCaller;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct App {
    app_id: H256,
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
    owner: Address,
    pin_to_nodes: Option<Vec<H256>>,
}

impl App {
    pub fn new(
        app_id: H256,
        storage_hash: H256,
        storage_receipt: H256,
        cluster_size: u8,
        owner: Address,
        pin_to_nodes: Option<Vec<H256>>,
    ) -> App {
        App {
            app_id,
            storage_hash,
            storage_receipt,
            cluster_size,
            owner,
            pin_to_nodes,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Cluster {
    app: App,
    genesis_time: u32,
    node_ids: Vec<H256>,
    ports: Vec<u16>,
}

impl Cluster {
    pub fn new(app: App, genesis_time: u32, node_ids: Vec<H256>, ports: Vec<u16>) -> Cluster {
        Cluster {
            app,
            genesis_time,
            node_ids,
            ports,
        }
    }
}

pub fn get_enqueued_apps(contract: &ContractCaller) -> Result<Vec<App>, Box<Error>> {
    let (call_data, decoder) = get_enqueued_apps::call();
    let (storage_hashes, app_ids, cluster_sizes, owners, _, _) =
        contract.query_contract(call_data, Box::new(decoder))?;

    let mut apps: Vec<App> = Vec::new();
    for i in 0..storage_hashes.len() {
        // TODO: use try_into when Rust 1.33 is stable
        let cluster_size: u64 = cluster_sizes[i].into();

        let app = App::new(
            app_ids[i],
            storage_hashes[i],
            H256::zero(), // storage_receipts was deleted since it was wasting stack in Solidity and wasn't used yet
            cluster_size as u8,
            owners[i],
            None,
        );
        apps.push(app);
    }

    Ok(apps)
}

pub fn get_clusters(contract: &ContractCaller) -> Result<Vec<Cluster>, Box<Error>> {
    let (call_data, decoder) = get_app_i_ds::call();
    let app_ids: Vec<H256> = contract.query_contract(call_data, Box::new(decoder))?;

    let clusters: Result<Vec<Cluster>, Box<Error>> = app_ids
        .iter()
        .map(|id| {
            let (call_data, decoder) = get_cluster::call(*id);
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

            let cluster_size: u64 = cluster_size.into();

            let app = App::new(
                *id,
                storage_hash,
                storage_receipt,
                cluster_size as u8,
                owner,
                Some(pin_to),
            );

            let genesis: u64 = genesis.into();
            let ports = ports
                .iter()
                .map(|p| (Into::<u64>::into(*p) as u16))
                .collect();

            Ok(Cluster::new(app, genesis as u32, node_ids, ports))
        })
        .collect();

    Ok(clusters?)
}
