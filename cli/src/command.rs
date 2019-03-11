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

use crate::config::SetupConfig;
use crate::credentials;
use crate::credentials::Credentials;
use crate::ethereum_params::EthereumParams;
use crate::utils;
use crate::utils::parse_hex;
use clap::value_t;
use clap::Arg;
use clap::ArgMatches;
use failure::Error;
use failure::ResultExt;
use std::net::IpAddr;
use web3::types::Address;
use web3::types::H160;
use web3::types::H256;

const PASSWORD: &str = "password";
const SECRET_KEY: &str = "secret_key";
const KEYSTORE: &str = "keystore";
const GAS: &str = "gas";
const GAS_PRICE: &str = "gas_price";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const BASE64_TENDERMINT_KEY: &str = "base64_tendermint_key";
const TENDERMINT_NODE_ID: &str = "tendermint_node_id";
const WAIT: &str = "wait";
const WAIT_SYNCING: &str = "wait_syncing";
pub const NODE_IP: &str = "node_ip";
pub const TENDERMINT_KEY: &str = "tendermint_key";

pub const TO_GWEI_MUL: u64 = 1_000_000_000;

// TODO: merge EthereumArgs, SetupConfig and EthereumParams into a single structure
#[derive(Debug, Clone)]
pub struct EthereumArgs {
    pub credentials: Credentials,
    pub gas: u32,
    pub gas_price: u64,
    pub account: Option<Address>,
    pub contract_address: Option<Address>,
    pub eth_url: Option<String>,
    pub wait_tx_include: bool,
    pub wait_eth_sync: bool,
}

pub fn contract_address<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(CONTRACT_ADDRESS)
        .long(CONTRACT_ADDRESS)
        .short("d")
        .value_name("eth address")
        .takes_value(true)
        .help("Fluence contract address")
}

pub fn eth_url<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(ETH_URL)
        .long(ETH_URL)
        .short("e")
        .value_name("url")
        .required(false)
        .takes_value(true)
        .help("Http address to ethereum node")
}

pub fn tendermint_key<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(TENDERMINT_KEY)
        .long(TENDERMINT_KEY)
        .short("K")
        .value_name("key")
        .required(true)
        .takes_value(true)
        .help("Public key of tendermint node")
}

pub fn base64_tendermint_key<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(BASE64_TENDERMINT_KEY)
        .long(BASE64_TENDERMINT_KEY)
        .help("Allows to use base64 tendermint key")
}

pub fn tendermint_node_id<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(TENDERMINT_NODE_ID)
        .long(TENDERMINT_NODE_ID)
        .short("n")
        .required(true)
        .takes_value(true)
        .help("Tendermint node ID (20-byte from SHA of p2p public key)")
}

pub fn node_ip<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(NODE_IP)
        .long(NODE_IP)
        .value_name("ip address")
        .short("i")
        .required(true)
        .takes_value(true)
        .help("Node's IP address")
}

// Takes `args` and concatenates them with predefined set of arguments needed for
// interaction with Ethereum.
pub fn with_ethereum_args<'a, 'b>(args: &[Arg<'a, 'b>]) -> Vec<Arg<'a, 'b>> {
    let eth_args = vec![
        contract_address(),
        eth_url(),
        Arg::with_name(ACCOUNT)
            .long(ACCOUNT)
            .short("a")
            .value_name("eth address")
            .required(false)
            .takes_value(true)
            .help("Ethereum account"),
        Arg::with_name(PASSWORD)
            .long(PASSWORD)
            .short("P")
            .required(false)
            .takes_value(true)
            .help("Password to unlock account in ethereum client"),
        Arg::with_name(SECRET_KEY)
            .long(SECRET_KEY)
            .short("S")
            .required(false)
            .takes_value(true)
            .help("The secret key to sign transactions"),
        Arg::with_name(GAS)
            .long(GAS)
            .short("g")
            .required(false)
            .takes_value(true)
            .default_value("1000000")
            .help("Maximum gas to spend"),
        Arg::with_name(GAS_PRICE)
            .long(GAS_PRICE)
            .short("G")
            .required(false)
            .takes_value(true)
            .default_value("1")
            .help("Gas price in Gwei"),
        Arg::with_name(KEYSTORE)
            .long(KEYSTORE)
            .short("T")
            .value_name("path")
            .required(false)
            .takes_value(true)
            .help("Path to keystore JSON file with Ethereum private key inside"),
        Arg::with_name(WAIT)
            .long(WAIT)
            .short("W")
            .required(false)
            .takes_value(false)
            .help("If supplied, wait for the transaction to be included in a block"),
        Arg::with_name(WAIT_SYNCING)
            .long(WAIT_SYNCING)
            .help("If supplied, wait until Ethereum is synced with blockchain"),
    ];

    // display subcommand-specific options on top of ethereum options
    let mut eth_args: Vec<Arg> = eth_args.into_iter().map(|a| a.display_order(10)).collect();

    // append args
    eth_args.extend_from_slice(args);

    // sort so positional arguments are always at the end, to reverse them later (clap nuance)
    eth_args.sort_unstable_by_key(|a| a.index);

    // reverse so positional arguments are always at the beginning (clap nuance)
    eth_args.reverse();

    eth_args
}

pub fn parse_contract_address(args: &ArgMatches) -> Result<Option<Address>, Error> {
    Ok(parse_hex(args.value_of(CONTRACT_ADDRESS))?)
}

pub fn parse_eth_url(args: &ArgMatches) -> Option<String> {
    args.value_of(ETH_URL).map(|s| s.to_owned())
}

pub fn parse_ethereum_args(
    args: &ArgMatches,
    config: SetupConfig,
) -> Result<EthereumParams, Error> {
    let secret_key = utils::parse_secret_key(args.value_of(SECRET_KEY))?;
    let password = args.value_of(PASSWORD).map(|s| s.to_string());
    let keystore = args.value_of(KEYSTORE).map(|s| s.to_string());

    let credentials = credentials::load_credentials(keystore, password, secret_key)?;

    let gas = value_t!(args, GAS, u32)?;
    let gas_price = value_t!(args, GAS_PRICE, u64)?;
    // TODO: it could panic here on overflow
    let gas_price = gas_price * TO_GWEI_MUL;
    let account: Option<Address> = utils::parse_hex(args.value_of(ACCOUNT))?;
    let account = account.or(credentials.to_address());

    let contract_address: Option<Address> = parse_contract_address(args)?;

    let eth_url = parse_eth_url(args);

    let wait = args.is_present(WAIT);
    let wait_syncing = args.is_present(WAIT_SYNCING);

    let eth_args = EthereumArgs {
        credentials,
        gas,
        gas_price,
        account,
        contract_address,
        eth_url,
        wait_tx_include: wait,
        wait_eth_sync: wait_syncing,
    };

    Ok(EthereumParams::generate(eth_args, config)?)
}

pub fn parse_tendermint_key(args: &ArgMatches) -> Result<H256, Error> {
    let tendermint_key = utils::parse_hex_string(args, TENDERMINT_KEY)
        .context("error parsing tendermint key")?
        .to_owned();
    let base64 = args.is_present(BASE64_TENDERMINT_KEY);
    let tendermint_key = if base64 {
        let arr =
            base64::decode(&tendermint_key).context("error parsing tendermint key as base64")?;
        hex::encode(arr)
    } else {
        tendermint_key
    };

    let tendermint_key = tendermint_key.parse::<H256>();

    let tendermint_key = if base64 {
        tendermint_key.context(format!(
            "error parsing tendermint key, did you forgot --{}?",
            BASE64_TENDERMINT_KEY
        ))
    } else {
        tendermint_key.context(format!("error parsing tendermint key"))
    };

    Ok(tendermint_key?)
}

pub fn parse_tendermint_node_id(args: &ArgMatches) -> Result<H160, Error> {
    Ok(value_t!(args, TENDERMINT_NODE_ID, H160)?)
}

pub fn parse_node_ip(args: &ArgMatches) -> Result<IpAddr, Error> {
    Ok(value_t!(args, NODE_IP, IpAddr)?)
}

impl Default for EthereumArgs {
    fn default() -> EthereumArgs {
        EthereumArgs {
            credentials: Credentials::No,
            gas: 1_000_000,
            gas_price: 1_000_000_000,
            account: Some("4180FC65D613bA7E1a385181a219F1DBfE7Bf11d".parse().unwrap()),
            contract_address: Some("9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap()),
            eth_url: Some(String::from("http://localhost:8545")),
            wait_tx_include: false,
            wait_eth_sync: false,
        }
    }
}

impl EthereumArgs {
    pub fn with_acc_creds(account: Address, credentials: Credentials) -> EthereumArgs {
        let mut args = EthereumArgs::default();
        args.credentials = credentials;
        args.account = Some(account);
        args
    }
}
