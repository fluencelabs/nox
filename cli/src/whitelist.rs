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

use web3::types::{Address, H256};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::error::Error;
use utils;

const ADDRESS_TO_ADD: &str = "address";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const PASSWORD: &str = "password";

#[derive(Debug)]
pub struct AddToWhitelist {
    account: Address,
    address_to_add: Address,
    contract_address: Address,
    eth_url: String,
    password: Option<String>
}

impl AddToWhitelist {
    pub fn new(
        account: Address,
        address_to_add: Address,
        contract_address: Address,
        eth_url: String,
        password: Option<String>,
    ) -> AddToWhitelist {
        AddToWhitelist {
            account,
            address_to_add,
            contract_address,
            eth_url,
            password
        }
    }

    pub fn add_to_whitelist(&self, show_progress: bool) -> Result<H256, Box<Error>> {
        let pass = self.password.as_ref().map(|s| s.as_str());

        let add_to_whitelist_fn = || -> Result<H256, Box<Error>> {
            utils::add_to_white_list(&self.eth_url, self.address_to_add, self.contract_address, self.account, pass)
        };

        let transaction = if show_progress {
            utils::with_progress(
                "Adding the address to the smart contract...",
                "1/1",
                "The address added.",
                add_to_whitelist_fn,
            )
        } else {
            add_to_whitelist_fn()
        };

        transaction
    }
}

pub fn parse(matches: &ArgMatches) -> Result<AddToWhitelist, Box<std::error::Error>> {

    let address_to_add = matches
        .value_of(ADDRESS_TO_ADD)
        .unwrap()
        .trim_left_matches("0x");
    let address_to_add: Address = address_to_add.parse()?;

    let contract_address = matches
        .value_of(CONTRACT_ADDRESS)
        .unwrap()
        .trim_left_matches("0x");
    let contract_address: Address = contract_address.parse()?;

    let account = matches.value_of(ACCOUNT).unwrap().trim_left_matches("0x");
    let account: Address = account.parse()?;

    let eth_url = matches.value_of(ETH_URL).unwrap().to_string();

    let password = matches.value_of(PASSWORD).map(|s| s.to_string());

    Ok(AddToWhitelist::new(
        account,
        address_to_add,
        contract_address,
        eth_url,
        password
    ))
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("add-to-whitelist")
        .about("Adds an address to the whitelist of Fluence smart contract.")
        .args(&[
            Arg::with_name(ADDRESS_TO_ADD)
                .alias(ADDRESS_TO_ADD)
                .required(true)
                .index(1)
                .takes_value(true)
                .help("address to add"),
            Arg::with_name(ACCOUNT)
                .alias(ACCOUNT)
                .required(true)
                .takes_value(true)
                .index(2)
                .help("your ethereum account"),
            Arg::with_name(CONTRACT_ADDRESS)
                .alias(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .index(3)
                .help("deployer contract address"),
            Arg::with_name(PASSWORD)
                .alias(PASSWORD)
                .takes_value(true)
                .long("password")
                .short("p")
                .required(false)
                .help("password to unlock account in ethereum client"),
            Arg::with_name(ETH_URL)
                .alias(ETH_URL)
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/")
        ])
}
