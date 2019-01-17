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

use std::error::Error;

use derive_getters::Getters;
use serde_derive::{Deserialize, Serialize};
use web3::types::{H256, U256};

use crate::contract_func::ContractCaller;
use crate::contract_status::app::App;

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Worker {
    node_id: H256,
    port: u16,
}

impl Worker {
    pub fn new(node_id: H256, port: u16) -> Result<Worker, Box<Error>> {
        Ok(Worker { node_id, port })
    }
}

#[derive(Serialize, Deserialize, Debug, Getters)]
pub struct Cluster {
    id: H256,
    app: App,
    genesis_time: U256,
    workers: Vec<Worker>,
}

impl Cluster {
    pub fn new(id: H256, app: App, genesis_time: U256, workers: Vec<Worker>) -> Cluster {
        Cluster {
            id,
            genesis_time,
            app,
            workers,
        }
    }
}

/// Gets list of formed clusters from Fluence contract
// TODO: implement
pub fn get_clusters(_contract: &ContractCaller) -> Result<Vec<Cluster>, Box<Error>> {
    let clusters: Vec<Cluster> = Vec::new();
    Ok(clusters)
}
