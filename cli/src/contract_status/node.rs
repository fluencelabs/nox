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
use types::NodeAddress;
use web3::types::H256;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Node {
    id: H256,
    tendermint_key: String,
    ip_addr: String,
    start_port: u16,
    end_port: u16,
    current_port: u16,
}

impl Node {
    pub fn new(
        id: H256,
        address: NodeAddress,
        start_port: u16,
        end_port: u16,
        current_port: u16,
    ) -> Result<Node, Box<Error>> {
        let (tendermint_key, ip_addr) = address.decode()?;
        Ok(Node {
            id,
            tendermint_key,
            ip_addr,
            start_port,
            end_port,
            current_port,
        })
    }
}
