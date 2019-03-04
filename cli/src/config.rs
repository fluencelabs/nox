use ethkey::Secret;
use failure::Error;
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::fs::create_dir_all;
use std::fs::File;
use std::io::prelude::*;
use web3::types::Address;
use web3::types::H256;

pub const HOME_DIR: &str = "~/.local/share/.fluence/";

#[derive(Serialize, Deserialize)]
pub struct CliConfig {
    contract_address: Address,
    account: Address,
    eth_url: String,
    swarm_url: String,
    secrey_key: Option<H256>,
    keystore_path: Option<String>,
    password: Option<String>,
}

impl CliConfig {
    pub fn new(
        contract_address: Address,
        account: Address,
        eth_url: String,
        swarm_url: String,
        secrey_key: Option<H256>,
        keystore_path: Option<String>,
        password: Option<String>,
    ) -> CliConfig {
        return CliConfig {
            contract_address,
            account,
            eth_url,
            swarm_url,
            secrey_key,
            keystore_path,
            password,
        };
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
