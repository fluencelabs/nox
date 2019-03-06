/*
 * Copyright 2019 Fluence Labs Limited
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

use failure::Error;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::PathBuf;
use web3::types::Address;
use web3::types::H256;

pub const FLUENCE_DIR: &str = ".fluence/";
pub const DEFAULT_CONTRACT_ADDRESS: &str = include_str!("../../tools/deploy/scripts/contract.txt");
pub const DEFAULT_ETH_URL: &str = "http://data.fluence.one:8545/";
pub const DEFAULT_SWARM_URL: &str = "http://data.fluence.one:8500/";

const CONFIG_FILENAME: &str = "cli.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct SetupConfig {
    pub contract_address: Address,
    pub account: Option<Address>,
    pub eth_url: String,
    pub swarm_url: String,
    pub secret_key: Option<H256>,
    pub keystore_path: Option<String>,
    pub password: Option<String>,
}

// returns None if string value is empty and Some if non empty
pub fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn get_config_dir() -> PathBuf {
    let mut home = dirs::data_local_dir().unwrap();
    home.push(FLUENCE_DIR);
    home
}

impl SetupConfig {
    pub fn new(
        contract_address: Address,
        account: Option<Address>,
        eth_url: String,
        swarm_url: String,
        secret_key: Option<H256>,
        keystore_path: Option<String>,
        password: Option<String>,
    ) -> SetupConfig {
        return SetupConfig {
            contract_address,
            account,
            eth_url,
            swarm_url,
            secret_key,
            keystore_path,
            password,
        };
    }

    pub fn default() -> Result<SetupConfig, Error> {
        let contract: Address = DEFAULT_CONTRACT_ADDRESS
            .to_owned()
            .trim_start_matches("0x")
            .parse()?;
        let eth_url = DEFAULT_ETH_URL.to_owned();
        let swarm_url = DEFAULT_SWARM_URL.to_owned();
        Ok(SetupConfig::new(
            contract, None, eth_url, swarm_url, None, None, None,
        ))
    }

    // reads config file or generates default config if file does not exist
    pub fn read_from_file_or_default() -> Result<SetupConfig, Error> {
        let mut path = get_config_dir();
        path.push(CONFIG_FILENAME);

        if !path.exists() {
            SetupConfig::default()
        } else {
            let content = read_to_string(path)?;
            let config: SetupConfig = serde_json::from_str(content.as_str())?;
            Ok(config)
        }
    }

    // writes config to file
    pub fn write_to_file(&self) -> Result<(), Error> {
        let mut path = get_config_dir();

        create_dir_all(&path)?;

        let config_str = serde_json::to_string(&self)?;

        path.push(CONFIG_FILENAME);
        println!("path: {:?}", &path);
        let mut file = File::create(&path)?;

        file.write_all(config_str.as_bytes())?;

        Ok(())
    }
}
