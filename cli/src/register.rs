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

use clap::{App, Arg, ArgMatches, SubCommand};
use ethabi::Token;
use hex;
use std::boxed::Box;
use std::error::Error;
use std::net::IpAddr;
use utils;
use web3::contract::tokens::Tokenizable;
use web3::contract::{Error as ContractError, ErrorKind};
use web3::types::{Address, H256};

const ADDRESS: &str = "address";
const TENDERMINT_KEY: &str = "tendermint_key";
const MIN_PORT: &str = "min_port";
const MAX_PORT: &str = "max_port";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const PASSWORD: &str = "password";

/// number of bytes for encoding an IP address
const IP_LEN: usize = 4;

/// number of bytes for encoding tendermint key
const TENDERMINT_KEY_LEN: usize = 20;

/// number of bytes for encoding IP address and tendermint key
const NODE_ADDR_LEN: usize = IP_LEN + TENDERMINT_KEY_LEN;
construct_fixed_hash!{ pub struct H192(NODE_ADDR_LEN); }

/// Helper for converting the hash structure to web3 format
impl Tokenizable for H192 {
    fn from_token(token: Token) -> Result<Self, ContractError> {
        match token {
            Token::FixedBytes(mut s) => {
                if s.len() != NODE_ADDR_LEN {
                    bail!(ErrorKind::InvalidOutputType(format!(
                        "Expected `H192`, got {:?}",
                        s
                    )));
                }
                let mut data = [0; NODE_ADDR_LEN];
                for (idx, val) in s.drain(..).enumerate() {
                    data[idx] = val;
                }
                Ok(data.into())
            }
            other => Err(
                ErrorKind::InvalidOutputType(format!("Expected `H192`, got {:?}", other)).into(),
            ),
        }
    }

    fn into_token(self) -> Token {
        Token::FixedBytes(self.0.to_vec())
    }
}

#[derive(Debug)]
pub struct Register {
    node_ip: IpAddr,
    tendermint_key: H256,
    min_port: u16,
    max_port: u16,
    contract_address: Address,
    account: Address,
    eth_url: String,
    password: Option<String>,
}

impl Register {
    /// Creates `Register` structure
    pub fn new(
        node_address: IpAddr,
        tendermint_key: H256,
        min_port: u16,
        max_port: u16,
        contract_address: Address,
        account: Address,
        eth_url: String,
        password: Option<String>,
    ) -> Result<Register, Box<Error>> {
        if max_port < min_port {
            let err: Box<Error> = From::from("max_port should be bigger than min_port".to_string());
            return Err(err);
        }

        Ok(Register {
            node_ip: node_address,
            tendermint_key,
            min_port,
            max_port,
            contract_address,
            account,
            eth_url,
            password,
        })
    }

    /// Serializes a node IP address and a tendermint key into the hash of node's key address
    fn serialize_node_address(&self) -> Result<H192, Box<Error>> {
        let ip_str = self.node_ip.to_string();
        let split = ip_str.split(".");

        let mut addr_bytes: [u8; 4] = [0; IP_LEN];

        for (i, part) in split.enumerate() {
            addr_bytes[i] = part.parse()?;
        }

        let mut addr_vec = addr_bytes.to_vec();

        let key_str = format!("{:?}", &self.tendermint_key);
        let key_str = key_str.as_str().trim_left_matches("0x");

        let key_bytes = hex::decode(key_str.to_owned())?;
        let mut key_bytes = key_bytes.as_slice()[0..20].to_vec();
        &mut key_bytes.append(&mut addr_vec);

        let serialized = hex::encode(key_bytes);

        let hash_addr: H192 = serialized.parse()?;

        Ok(hash_addr)
    }

    /// Registers a node in Fluence smart contract
    pub fn register(&self, show_progress: bool) -> Result<H256, Box<Error>> {
        let publish_to_contract_fn = || -> Result<H256, Box<Error>> {
            let pass = self.password.as_ref().map(|s| s.as_str());

            let options = utils::options_with_gas(300_000);

            let hash_addr = self.serialize_node_address()?;

            utils::call_contract(
                self.account,
                self.contract_address,
                pass,
                &self.eth_url,
                "addNode",
                (
                    self.tendermint_key,
                    hash_addr,
                    self.min_port as u64,
                    self.max_port as u64,
                ),
                options,
            )
        };

        // sending transaction with the hash of file with code to ethereum
        let transaction = if show_progress {
            utils::with_progress(
                "Adding a solver to the smart contract...",
                "1/1",
                "The solver added.",
                publish_to_contract_fn,
            )
        } else {
            publish_to_contract_fn()
        };

        transaction
    }
}

pub fn parse(matches: &ArgMatches) -> Result<Register, Box<Error>> {
    let node_address: IpAddr = matches.value_of(ADDRESS).unwrap().parse()?;

    let tendermint_key = matches
        .value_of(TENDERMINT_KEY)
        .unwrap()
        .trim_left_matches("0x");

    let tendermint_key: H256 = tendermint_key.parse()?;

    let min_port: u16 = matches.value_of(MIN_PORT).unwrap().parse()?;
    let max_port: u16 = matches.value_of(MAX_PORT).unwrap().parse()?;

    let contract_address = matches
        .value_of(CONTRACT_ADDRESS)
        .unwrap()
        .trim_left_matches("0x");
    let contract_address: Address = contract_address.parse()?;

    let account = matches.value_of(ACCOUNT).unwrap().trim_left_matches("0x");
    let account: Address = account.parse()?;

    let eth_url = matches.value_of(ETH_URL).unwrap().to_string();

    let password = matches.value_of(PASSWORD).map(|s| s.to_string());

    Register::new(
        node_address,
        tendermint_key,
        min_port,
        max_port,
        contract_address,
        account,
        eth_url,
        password,
    )
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("register")
        .about("Register solver in smart contract")
        .args(&[
            Arg::with_name(ADDRESS)
                .alias(ADDRESS)
                .required(true)
                .index(1)
                .takes_value(true)
                .help("node's IP address"),
            Arg::with_name(TENDERMINT_KEY)
                .alias(TENDERMINT_KEY)
                .required(true)
                .index(2)
                .takes_value(true)
                .help("public key of tendermint node"),
            Arg::with_name(ACCOUNT)
                .alias(ACCOUNT)
                .required(true)
                .takes_value(true)
                .index(3)
                .help("ethereum account"),
            Arg::with_name(CONTRACT_ADDRESS)
                .alias(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .index(4)
                .help("deployer contract address"),
            Arg::with_name(MIN_PORT)
                .alias(MIN_PORT)
                .default_value("20096")
                .takes_value(true)
                .help("minimum port in the port range"),
            Arg::with_name(MAX_PORT)
                .alias(MAX_PORT)
                .default_value("20196")
                .takes_value(true)
                .help("maximum port in the port range"),
            Arg::with_name(ETH_URL)
                .alias(ETH_URL)
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
            Arg::with_name(PASSWORD)
                .alias(PASSWORD)
                .long("password")
                .short("p")
                .required(false)
                .takes_value(true)
                .help("password to unlock account in ethereum client"),
        ])
}

#[cfg(test)]
mod tests {
    use super::Register;
    use rand::prelude::*;
    use std::error::Error;
    use utils;
    use web3;
    use web3::futures::Future;
    use web3::types::*;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

    fn generate_register() -> Register {
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
            None,
        ).unwrap()
    }

    pub fn generate_with<F>(func: F) -> Register
    where
        F: FnOnce(&mut Register),
    {
        let mut register = generate_register();
        func(&mut register);
        register
    }

    pub fn generate_with_account(account: Address) -> Register {
        generate_with(|r| {
            r.account = account;
        })
    }

    #[test]
    fn register_success() -> Result<(), Box<Error>> {
        let register = generate_with_account("02f906f8b3b932fd282109a5b8dc732ba2329888".parse()?);

        let pass = register.password.as_ref().map(|s| s.as_str());

        utils::add_to_white_list(
            &register.eth_url,
            register.account,
            register.contract_address,
            OWNER.parse()?,
            pass,
        )?;

        register.register(false)?;

        Ok(())
    }
}
