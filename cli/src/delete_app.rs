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

extern crate clap;

use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use contract_func::contract::functions::delete_app;
use contract_func::contract::functions::dequeue_app;
use contract_func::ContractCaller;
use credentials::Credentials;
use std::boxed::Box;
use std::error::Error;
use utils;
use web3::types::{Address, H256};

const APP_ID: &str = "app_id";
const CLUSTER_ID: &str = "cluster_id";
const PASSWORD: &str = "password";
const SECRET_KEY: &str = "secret_key";
const GAS: &str = "gas";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";

#[derive(Debug)]
pub struct DeleteApp {
    app_id: H256,
    cluster_id: Option<H256>,
    credentials: Credentials,
    gas: u32,
    account: Address,
    contract_address: Address,
    eth_url: String,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("delete_app")
        .about("Delete app from smart-contract")
        .args(&[
            Arg::with_name(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .index(1)
                .help("fluence contract address"),
            Arg::with_name(ACCOUNT)
                .required(true)
                .index(2)
                .takes_value(true)
                .help("ethereum account"),
            Arg::with_name(ETH_URL)
                .long(ETH_URL)
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
            Arg::with_name(APP_ID)
                .required(true)
                .takes_value(true)
                .help("app to be removed"),
            Arg::with_name(CLUSTER_ID)
                .required(false)
                .takes_value(true)
                .help("ID of cluster hosting the app"),
            Arg::with_name(PASSWORD)
                .long(PASSWORD)
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
            Arg::with_name(GAS)
                .long(GAS)
                .short("g")
                .required(false)
                .takes_value(true)
                .default_value("1000000")
                .help("maximum gas to spend"),
        ])
}

pub fn parse(args: &ArgMatches) -> Result<DeleteApp, Box<Error>> {
    let app_id: H256 = utils::parse_hex_opt(args, APP_ID)?.parse()?;
    let cluster_id = args
        .value_of(CLUSTER_ID)
        .map(|v| v.trim_start_matches("0x").parse::<H256>())
        .map_or(Ok(None), |r| r.map(Some).into())?;

    let secret_key = utils::parse_secret_key(args, SECRET_KEY)?;
    let password = args.value_of(PASSWORD).map(|s| s.to_string());

    let credentials = Credentials::get(secret_key, password);

    let gas = value_t!(args, GAS, u32)?;
    let account: Address = utils::parse_hex_opt(args, ACCOUNT)?.parse()?;

    let contract_address: Address = utils::parse_hex_opt(args, CONTRACT_ADDRESS)?.parse()?;

    let eth_url = value_t!(args, ETH_URL, String)?;

    return Ok(DeleteApp {
        app_id,
        cluster_id,
        credentials,
        gas,
        account,
        contract_address,
        eth_url,
    });
}

impl DeleteApp {
    pub fn delete_app(self, show_progress: bool) -> Result<H256, Box<Error>> {
        let delete_app_fn = || -> Result<H256, Box<Error>> {
            let call_data = match self.cluster_id {
                Some(cluster_id) => delete_app::call(self.app_id, cluster_id).0,
                None => dequeue_app::call(self.app_id).0,
            };

            let contract = ContractCaller::new(self.contract_address, &self.eth_url)?;

            contract.call_contract(self.account, &self.credentials, call_data, self.gas)
        };

        if show_progress {
            utils::with_progress(
                "Deleting app from smart contract...",
                "1/1",
                "App deleted.",
                delete_app_fn,
            )
        } else {
            delete_app_fn()
        }
    }
}
