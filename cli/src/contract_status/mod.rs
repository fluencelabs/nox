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

mod cluster;
mod code;
mod node;
mod status;

use self::status::{get_clusters, get_enqueued_codes, get_ready_nodes, Status};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::boxed::Box;
use std::error::Error;
use web3::types::Address;

const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("status")
        .about("Get status of smart contract")
        .args(&[
            Arg::with_name(CONTRACT_ADDRESS)
                .alias(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .help("deployer contract address"),
            Arg::with_name(ETH_URL)
                .alias(ETH_URL)
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
        ])
}

pub fn get_status_by_args(args: &ArgMatches) -> Result<Status, Box<Error>> {
    let contract_address: Address = args
        .value_of(CONTRACT_ADDRESS)
        .unwrap()
        .trim_left_matches("0x")
        .parse()?;
    let eth_url: &str = args.value_of(ETH_URL).unwrap();

    get_status(contract_address, eth_url)
}

pub fn get_status(contract_address: Address, eth_url: &str) -> Result<Status, Box<Error>> {
    let nodes = get_ready_nodes(contract_address, eth_url)?;

    let clusters = get_clusters(contract_address, eth_url)?;
    let codes = get_enqueued_codes(contract_address, eth_url)?;

    Ok(Status::new(clusters, codes, nodes))
}
