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
use std::boxed::Box;
use std::error::Error;
use web3::types::{Address, H256};

const NODE_ID: &str = "node_id";
const ADDRESS: &str = "address";
const MIN_PORT: &str = "min_port";
const MAX_PORT: &str = "max_port";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const PASSWORD: &str = "password";

pub struct Register {
    pub node_id: String,
    pub node_address: String,
    pub min_port: u16,
    pub max_port: u16,
    pub contract_address: Address,
    pub account: Address,
    pub eth_url: String,
    pub password: Option<String>,
}

impl Register {
    pub fn new(
        node_id: String,
        node_address: String,
        min_port: u16,
        max_port: u16,
        contract_address: Address,
        account: Address,
        eth_url: String,
        password: Option<String>,
    ) -> Register {
        if max_port < min_port {
            panic!("max_port should be bigger than min_port");
        }

        Register {
            node_id,
            node_address,
            min_port,
            max_port,
            contract_address,
            account,
            eth_url,
            password,
        }
    }

    pub fn register() -> Result<H256, Box<Error>> {
        let h: H256 = "".parse()?;
        Ok(h)
    }
}

pub fn parse(matches: ArgMatches) -> Result<Register, Box<std::error::Error>> {
    let node_id = matches.value_of(NODE_ID).unwrap().to_string();
    let node_address = matches.value_of(ADDRESS).unwrap().to_string();
    let min_port: u16 = matches.value_of(MIN_PORT).unwrap().parse().unwrap();
    let max_port: u16 = matches.value_of(MAX_PORT).unwrap().parse().unwrap();

    let contract_address = matches
        .value_of("contract_address")
        .unwrap()
        .trim_left_matches("0x");
    let contract_address: Address = contract_address.parse()?;

    let account = matches.value_of("account").unwrap().trim_left_matches("0x");
    let account: Address = account.parse()?;

    let eth_url = matches.value_of("eth_url").unwrap().to_string();

    let password = matches.value_of("password").map(|s| s.to_string());

    Ok(Register::new(
        node_id,
        node_address,
        min_port,
        max_port,
        contract_address,
        account,
        eth_url,
        password,
    ))
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("register")
        .about("Register solver in smart contract.")
        .args(&[
            Arg::with_name(NODE_ID)
                .alias(NODE_ID)
                .required(true)
                .index(1)
                .takes_value(true)
                .help("node's IP address"),
            Arg::with_name(ADDRESS)
                .alias(ADDRESS)
                .required(true)
                .index(2)
                .takes_value(true)
                .help("node's IP address"),
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
