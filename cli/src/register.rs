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

use std::boxed::Box;
use std::error::Error;
use std::net::IpAddr;
use std::{thread, time};

use clap::{value_t, App, Arg, ArgMatches, SubCommand};
use derive_getters::Getters;
use hex;
use web3::transports::Http;
use web3::types::{Address, H256};

use crate::contract_func::contract::functions::add_node;
use crate::contract_func::ContractCaller;
use crate::credentials::Credentials;
use crate::types::{NodeAddress, IP_LEN, TENDERMINT_KEY_LEN};
use crate::utils;

const ADDRESS: &str = "address";
const TENDERMINT_KEY: &str = "tendermint_key";
const START_PORT: &str = "start_port";
const LAST_PORT: &str = "last_port";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const PASSWORD: &str = "password";
const SECRET_KEY: &str = "secret_key";
const WAIT_SYNCING: &str = "wait_syncing";
const BASE64_TENDERMINT_KEY: &str = "base64_tendermint_key";
const GAS: &str = "gas";
const PRIVATE: &str = "private";

#[derive(Debug, Getters)]
pub struct Register {
    node_ip: IpAddr,
    tendermint_key: H256,
    start_port: u16,
    last_port: u16,
    contract_address: Address,
    account: Address,
    eth_url: String,
    credentials: Credentials,
    wait_syncing: bool,
    gas: u32,
    private: bool,
}

impl Register {
    /// Creates `Register` structure
    pub fn new(
        node_address: IpAddr,
        tendermint_key: H256,
        start_port: u16,
        last_port: u16,
        contract_address: Address,
        account: Address,
        eth_url: String,
        credentials: Credentials,
        wait_syncing: bool,
        gas: u32,
        private: bool,
    ) -> Result<Register, Box<Error>> {
        if last_port < start_port {
            let err: Box<Error> =
                From::from("last_port should be bigger than start_port".to_string());
            return Err(err);
        }

        Ok(Register {
            node_ip: node_address,
            tendermint_key,
            start_port,
            last_port,
            contract_address,
            account,
            eth_url,
            credentials,
            wait_syncing,
            gas,
            private,
        })
    }

    /// Serializes a node IP address and a tendermint key into the hash of node's key address
    fn serialize_node_address(&self) -> Result<NodeAddress, Box<Error>> {
        // serialize tendermint key
        let key_str = format!("{:?}", &self.tendermint_key);
        let key_str = key_str.as_str().trim_start_matches("0x");

        let key_bytes = hex::decode(key_str.to_owned())?;
        let mut key_bytes = key_bytes.as_slice()[0..TENDERMINT_KEY_LEN].to_vec();

        // serialize IP address
        let ip_str = self.node_ip.to_string();
        let split = ip_str.split('.');
        let mut addr_bytes: [u8; IP_LEN] = [0; IP_LEN];
        for (i, part) in split.enumerate() {
            addr_bytes[i] = part.parse()?;
        }
        let mut addr_vec = addr_bytes.to_vec();

        // concatenate tendermint key and IP address
        key_bytes.append(&mut addr_vec);

        let serialized = hex::encode(key_bytes);

        let hash_addr: NodeAddress = serialized.parse()?;

        Ok(hash_addr)
    }

    /// Registers a node in Fluence smart contract
    pub fn register(&self, show_progress: bool) -> Result<H256, Box<Error>> {
        let wait_syncing_fn = || -> Result<(), Box<Error>> {
            let (_eloop, transport) = Http::new(&self.eth_url)?;
            let web3 = &web3::Web3::new(transport);

            let mut sync = utils::check_sync(web3)?;

            let ten_seconds = time::Duration::from_secs(10);

            while sync {
                thread::sleep(ten_seconds);

                sync = utils::check_sync(web3)?;
            }

            Ok(())
        };

        let publish_to_contract_fn = || -> Result<H256, Box<Error>> {
            let hash_addr = self.serialize_node_address()?;

            let contract = ContractCaller::new(self.contract_address, &self.eth_url)?;

            let (call_data, _) = add_node::call(
                self.tendermint_key,
                hash_addr,
                u64::from(self.start_port),
                u64::from(self.last_port),
                self.private,
            );

            contract.call_contract(self.account, &self.credentials, call_data, self.gas)
        };

        // sending transaction with the hash of file with code to ethereum
        if show_progress {
            if self.wait_syncing {
                utils::with_progress(
                    "Waiting for the node is syncing",
                    "1/2",
                    "Node synced.",
                    wait_syncing_fn,
                )?;
            };

            let prefix = if self.wait_syncing { "2/2" } else { "1/1" };
            utils::with_progress(
                "Adding the node to the smart contract...",
                prefix,
                "Node added.",
                publish_to_contract_fn,
            )
        } else {
            publish_to_contract_fn()
        }
    }
}

pub fn parse(matches: &ArgMatches) -> Result<Register, Box<Error>> {
    let node_address: IpAddr = value_t!(matches, ADDRESS, IpAddr)?;

    let tendermint_key = utils::parse_hex_opt(matches, TENDERMINT_KEY)?.to_owned();
    let tendermint_key = if matches.is_present(BASE64_TENDERMINT_KEY) {
        let arr = base64::decode(&tendermint_key)?;
        hex::encode(arr)
    } else {
        tendermint_key
    };

    let tendermint_key: H256 = tendermint_key
        .parse()
        .map_err(|e| format!("error parsing tendermint key: {}", e))?;

    let start_port = value_t!(matches, START_PORT, u16)?;
    let last_port = value_t!(matches, LAST_PORT, u16)?;

    let contract_address: Address = utils::parse_hex_opt(matches, CONTRACT_ADDRESS)?.parse()?;

    let account: Address = utils::parse_hex_opt(matches, ACCOUNT)?.parse()?;

    let eth_url = value_t!(matches, ETH_URL, String)?;

    let secret_key = utils::parse_secret_key(matches, SECRET_KEY)?;

    let password = matches.value_of(PASSWORD).map(|s| s.to_string());

    let credentials = Credentials::get(secret_key, password);

    let wait_syncing = matches.is_present(WAIT_SYNCING);

    let gas = value_t!(matches, GAS, u32)?;

    let private: bool = matches.is_present(PRIVATE);

    Register::new(
        node_address,
        tendermint_key,
        start_port,
        last_port,
        contract_address,
        account,
        eth_url,
        credentials,
        wait_syncing,
        gas,
        private,
    )
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("register")
        .about("Register a node in the smart contract")
        .args(&[
            Arg::with_name(ADDRESS)
                .required(true)
                .index(1)
                .takes_value(true)
                .help("node's IP address"),
            Arg::with_name(TENDERMINT_KEY)
                .required(true)
                .index(2)
                .takes_value(true)
                .help("public key of tendermint node"),
            Arg::with_name(ACCOUNT)
                .required(true)
                .takes_value(true)
                .index(3)
                .help("ethereum account"),
            Arg::with_name(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .index(4)
                .help("fluence contract address"),
            Arg::with_name(START_PORT)
                .alias(START_PORT)
                .long(START_PORT)
                .default_value("20096")
                .takes_value(true)
                .help("minimum port in the port range"),
            Arg::with_name(LAST_PORT)
                .alias(LAST_PORT)
                .default_value("20196")
                .long(LAST_PORT)
                .takes_value(true)
                .help("maximum port in the port range"),
            Arg::with_name(ETH_URL)
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
            Arg::with_name(PASSWORD)
                .long("password")
                .short("p")
                .required(false)
                .takes_value(true)
                .help("password to unlock account in ethereum client"),
            Arg::with_name(SECRET_KEY)
                .long(SECRET_KEY)
                .short("s")
                .required(false)
                .takes_value(true)
                .help("the secret key to sign transactions"),
            Arg::with_name(WAIT_SYNCING)
                .long(WAIT_SYNCING)
                .help("waits until ethereum node will be synced, executes a command after this"),
            Arg::with_name(BASE64_TENDERMINT_KEY)
                .long(BASE64_TENDERMINT_KEY)
                .help("allows to use base64 tendermint key"),
            Arg::with_name(GAS)
                .long(GAS)
                .short("g")
                .takes_value(true)
                .default_value("1000000")
                .help("maximum gas to spend"),
            Arg::with_name(PRIVATE)
                .long(PRIVATE)
                .short("P")
                .takes_value(false)
                .help("marks node as private, used for pinning apps to nodes"),
        ])
}

#[cfg(test)]
pub mod tests {
    use std::error::Error;

    use ethkey::Secret;
    use rand::prelude::*;
    use web3::types::*;

    use crate::credentials::Credentials;

    use super::Register;

    pub fn generate_register(credentials: Credentials) -> Register {
        let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap();

        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();

        let tendermint_key: H256 = H256::from(rnd_num);
        let account: Address = "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap();

        Register::new(
            "127.0.0.1".parse().unwrap(),
            tendermint_key,
            25006,
            25100,
            contract_address,
            account,
            String::from("http://localhost:8545/"),
            credentials,
            false,
            1_000_000,
            false,
        )
        .unwrap()
    }

    pub fn generate_with<F>(func: F, credentials: Credentials) -> Register
    where
        F: FnOnce(&mut Register),
    {
        let mut register = generate_register(credentials);
        func(&mut register);
        register
    }

    pub fn generate_with_account(account: Address, credentials: Credentials) -> Register {
        generate_with(
            |r| {
                r.account = account;
            },
            credentials,
        )
    }

    #[test]
    fn register_success() -> Result<(), Box<Error>> {
        let register = generate_with_account(
            "fa0de43c68bea2167181cd8a83f990d02a049336".parse()?,
            Credentials::No,
        );

        register.register(false)?;

        Ok(())
    }

    #[test]
    fn register_out_of_gas() -> Result<(), Box<Error>> {
        let register = generate_with(
            |r| {
                r.gas = 1;
            },
            Credentials::No,
        );

        let result = register.register(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn register_success_with_secret() -> Result<(), Box<Error>> {
        let secret_arr: H256 =
            "a349fe22d5c6f8ad3a1ad91ddb65e8946435b52254ce8c330f7ed796e83bfd92".parse()?;
        let secret = Secret::from(secret_arr);
        let register = generate_with_account(
            "dce48d51717ad5eb87fb56ff55ec609cf37b9aad".parse()?,
            Credentials::Secret(secret),
        );

        register.register(false)?;

        Ok(())
    }
}
