use failure::Error;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::Path;
use web3::types::Address;
use web3::types::H256;

pub const HOME_DIR: &str = "~/.local/share/.fluence/";
pub const DEFAULT_CONTRACT_ADDRESS: &str = include_str!("../../tools/deploy/scripts/contract.txt");
pub const DEFAULT_ETH_URL: &str = "http://data.fluence.ai:8545/";
pub const DEFAULT_SWARM_URL: &str = "http://data.fluence.ai:8500/";

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

pub fn empty_or_default(value: String, default: String) -> String {
    if value.is_empty() {
        default
    } else {
        value
    }
}

pub fn none_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
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

    pub fn read_from_file(home_dir: &str) -> Result<SetupConfig, Error> {
        let path = format!("{}{}", home_dir, "cli.json");
        let path = Path::new(path.as_str());

        if !path.exists() {
            let contract: Address = DEFAULT_CONTRACT_ADDRESS
                .to_owned()
                .trim_start_matches("0x")
                .parse()?;
            let eth_url = DEFAULT_ETH_URL.to_owned();
            let swarm_url = DEFAULT_ETH_URL.to_owned();
            Ok(SetupConfig::new(
                contract, None, eth_url, swarm_url, None, None, None,
            ))
        } else {
            let content = read_to_string(path)?;
            let config: SetupConfig = serde_json::from_str(content.as_str())?;
            Ok(config)
        }
    }

    pub fn write_to_file(&self, home_dir: &str) -> Result<(), Error> {
        create_dir_all(HOME_DIR)?;
        let config_str = serde_json::to_string(&self)?;
        let path = format!("{}{}", home_dir, "cli.json");
        println!("path: {}", path);
        let mut file = File::create(path)?;

        file.write_all(config_str.as_bytes())?;

        Ok(())
    }
}
