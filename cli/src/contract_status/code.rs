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
pub struct Code {
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
    developer: Address,
}

impl Code {
    pub fn new(
        storage_hash: H256,
        storage_receipt: H256,
        cluster_size: u8,
        developer: Address,
    ) -> Code {
        Code {
            storage_hash,
            storage_receipt,
            cluster_size,
            developer,
        }
    }
}

pub fn get_enqueued_codes(contract: &ContractCaller) -> Result<Vec<Code>, Box<Error>> {
    let (storage_hashes, storage_receipts, cluster_sizes, developers): (
        Vec<H256>,
        Vec<H256>,
        Vec<u64>,
        Vec<Address>,
    ) = contract.query_contract("getEnqueuedCodes", ())?;

    let mut codes: Vec<Code> = Vec::new();
    for i in 0..storage_hashes.len() {
        let code = Code::new(
            storage_hashes[i],
            storage_receipts[i],
            cluster_sizes[i] as u8,
            developers[i],
        );
        codes.push(code);
    }

    Ok(codes)
}
