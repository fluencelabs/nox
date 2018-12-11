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

use contract_status::code::Code;
use types::NodeAddress;
use web3::types::{H256, U256};

#[derive(Serialize, Deserialize, Debug)]
pub struct ClusterMember {
    id: H256,
    address: NodeAddress,
    port: u16,
}

impl ClusterMember {
    pub fn new(id: H256, address: NodeAddress, port: u16) -> ClusterMember {
        ClusterMember { id, address, port }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Cluster {
    id: H256,
    genesis_time: U256,
    code: Code,
    cluster_members: Vec<ClusterMember>,
}

impl Cluster {
    pub fn new(
        id: H256,
        genesis_time: U256,
        code: Code,
        cluster_members: Vec<ClusterMember>,
    ) -> Cluster {
        Cluster {
            id,
            genesis_time,
            code,
            cluster_members,
        }
    }
}
