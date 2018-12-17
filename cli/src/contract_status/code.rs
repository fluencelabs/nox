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

use web3::types::H256;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Code {
    storage_hash: H256,
    storage_receipt: H256,
    cluster_size: u8,
}

impl Code {
    pub fn new(storage_hash: H256, storage_receipt: H256, cluster_size: u8) -> Code {
        Code {
            storage_hash,
            storage_receipt,
            cluster_size,
        }
    }
}
