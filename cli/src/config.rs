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

use std::fs::create_dir_all;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::PathBuf;

use ethkey::Secret;
use failure::{err_msg, Error, ResultExt};
use serde::{Deserialize, Serialize};
use web3::types::Address;
use web3::types::H256;

use crate::credentials::Credentials;

pub const FLUENCE_DIR: &str = ".fluence/";
pub const DEFAULT_CONTRACT_ADDRESS: &str = include_str!("../../tools/deploy/scripts/contract.txt");
pub const DEFAULT_ETH_URL: &str = "http://data.fluence.one:8545/";
pub const DEFAULT_SWARM_URL: &str = "http://data.fluence.one:8500/";

const CONFIG_FILENAME: &str = "cli.json";

#[derive(Serialize, Deserialize, Clone)]
pub enum Auth {
    SecretKey,
    Keystore,
    None,
}

impl Default for Auth {
    fn default() -> Self {
        Auth::None
    }
}

impl Auth {
    pub fn first_letter(&self) -> &str {
        match self {
            Auth::None => "N",
            Auth::SecretKey => "S",
            Auth::Keystore => "K",
        }
    }
}

// TODO: merge EthereumArgs, SetupConfig and EthereumParams into a single structure
#[derive(Clone)]
pub struct SetupConfig {
    pub contract_address: Address,
    pub eth_url: String,
    pub swarm_url: String,
    pub credentials: Credentials,
}

/// Flat, plain JSON representation of the config
#[derive(Serialize, Deserialize, Clone)]
struct FlatConfig {
    pub contract_address: Address,
    pub eth_url: String,
    pub swarm_url: String,
    #[serde(default)]
    pub auth: Auth,
    pub secret_key: Option<H256>,
    pub keystore_path: Option<String>,
    pub password: Option<String>,
}

impl FlatConfig {
    fn from_setup(config: SetupConfig) -> FlatConfig {
        let (auth, secret_key, keystore_path, password) = match config.credentials {
            Credentials::No => (Auth::None, None, None, None),
            Credentials::Keystore {
                secret: _,
                path,
                password,
            } => (Auth::Keystore, None, Some(path), Some(password)),
            Credentials::Secret(secret) => (Auth::SecretKey, Some(secret), None, None),
        };

        let secret_key = secret_key.map(|s| *s);

        FlatConfig {
            contract_address: config.contract_address,
            eth_url: config.eth_url,
            swarm_url: config.swarm_url,
            auth,
            secret_key,
            keystore_path,
            password,
        }
    }
}

// returns None if string value is empty and Some if non empty
pub fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn none_if_empty_string(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn get_config_dir() -> Result<PathBuf, Error> {
    let mut home = dirs::data_local_dir()
        .ok_or(err_msg("Can't get data local dir. This shouldn't happen."))?;
    home.push(FLUENCE_DIR);
    Ok(home)
}

impl SetupConfig {
    pub fn new(
        contract_address: Address,
        eth_url: String,
        swarm_url: String,
        credentials: Credentials,
    ) -> SetupConfig {
        return SetupConfig {
            contract_address,
            eth_url,
            swarm_url,
            credentials,
        };
    }

    pub fn default() -> Result<SetupConfig, Error> {
        let contract: Address = DEFAULT_CONTRACT_ADDRESS
            .to_string()
            .trim_start_matches("0x")
            .parse()?;
        let eth_url = DEFAULT_ETH_URL.to_string();
        let swarm_url = DEFAULT_SWARM_URL.to_string();
        Ok(SetupConfig::new(
            contract,
            eth_url,
            swarm_url,
            Credentials::No,
        ))
    }

    // reads config file or generates default config if file does not exist
    pub fn read_from_file_or_default() -> Result<SetupConfig, Error> {
        let mut path = get_config_dir()?;
        path.push(CONFIG_FILENAME);

        if !path.exists() {
            SetupConfig::default()
        } else {
            let content = read_to_string(path)?;
            let config: FlatConfig =
                serde_json::from_str(&content).context("Error while loading config")?;
            SetupConfig::from_flat(config)
        }
    }

    // writes config to file
    // TODO: encrypt config
    pub fn write_to_file(self) -> Result<(), Error> {
        let mut path = get_config_dir()?;

        create_dir_all(&path)?;

        let flat_config = FlatConfig::from_setup(self);
        let config_str = serde_json::to_string_pretty(&flat_config)?;

        path.push(CONFIG_FILENAME);
        println!("path: {:?}", &path);
        let mut file = File::create(&path)?;

        file.write_all(config_str.as_bytes())?;

        Ok(())
    }

    fn from_flat(config: FlatConfig) -> Result<Self, Error> {
        let credentials = match config.auth {
            Auth::Keystore => {
                let path = config
                    .keystore_path
                    .ok_or(err_msg("Auth::Keystore, but keystore path isn't defined"))?;
                let password = config
                    .password
                    .ok_or(err_msg("Auth::Keystore, but password isn't defined"))?;
                Credentials::load_keystore(path, password)?
            }
            Auth::SecretKey => {
                let secret = config
                    .secret_key
                    .ok_or(err_msg("Auth::SecretKey, but secret key isn't defined"))?;
                let secret = Secret::from(secret);
                Credentials::from_secret(secret)?
            }
            Auth::None => Credentials::No,
        };

        Ok(SetupConfig {
            contract_address: config.contract_address,
            eth_url: config.eth_url,
            swarm_url: config.swarm_url,
            credentials,
        })
    }
}
