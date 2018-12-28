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
use web3::types::{Address, H256};

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct App {
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
    owner: Address,
    pin_to_nodes: Option<Vec<H256>>,
}

impl App {
    pub fn new(
        storage_hash: H256,
        storage_receipt: H256,
        cluster_size: u8,
        owner: Address,
        pin_to_nodes: Option<Vec<H256>>,
    ) -> App {
        App {
            storage_hash,
            storage_receipt,
            cluster_size,
            owner,
            pin_to_nodes,
        }
    }
}

pub fn get_enqueued_apps(contract: &ContractCaller) -> Result<Vec<App>, Box<Error>> {
    let (storage_hashes, storage_receipts, cluster_sizes, owners): (
        Vec<H256>,
        Vec<H256>,
        Vec<u64>,
        Vec<Address>,
    ) = contract.query_contract("getEnqueuedApps", ())?;

    let mut apps: Vec<App> = Vec::new();
    for i in 0..storage_hashes.len() {
        let code = App::new(
            storage_hashes[i],
            storage_receipts[i],
            cluster_sizes[i] as u8,
            owners[i],
            None,
        );
        apps.push(code);
    }

    Ok(apps)
}
