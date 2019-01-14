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
