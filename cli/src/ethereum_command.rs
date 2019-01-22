use std::boxed::Box;
use std::error::Error;

use clap::value_t;
use clap::Arg;
use clap::ArgMatches;
use web3::types::Address;

use crate::credentials::Credentials;
use crate::utils;

const PASSWORD: &str = "password";
const SECRET_KEY: &str = "secret_key";
const GAS: &str = "gas";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";

#[derive(Debug)]
pub struct EthereumArgs {
    pub credentials: Credentials,
    pub gas: u32,
    pub account: Address,
    pub contract_address: Address,
    pub eth_url: String,
}

pub fn contract_address<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(CONTRACT_ADDRESS)
        .required(true)
        .takes_value(true)
        .index(1)
        .help("fluence contract address")
}

pub fn eth_url<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(ETH_URL)
        .long(ETH_URL)
        .short("e")
        .required(false)
        .takes_value(true)
        .help("http address to ethereum node")
        .default_value("http://localhost:8545/")
}

//pub fn contract_address<'a, 'b>() -> Arg<'a, 'b>

pub fn ethereum_args<'a, 'b>() -> [Arg<'a, 'b>; 6] {
    [
        contract_address(),
        Arg::with_name(ACCOUNT)
            .required(true)
            .index(2)
            .takes_value(true)
            .help("ethereum account"),
        eth_url(),
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
    ]
}

pub fn parse_contract_address(args: &ArgMatches) -> Result<Address, Box<Error>> {
    utils::parse_hex_opt(args, CONTRACT_ADDRESS)?.parse()?
}

pub fn parse_eth_url(args: &ArgMatches) -> Result<String, Box<Error>> {
    value_t!(args, ETH_URL, String)?
}

pub fn ethereum_parse(args: &ArgMatches) -> Result<EthereumArgs, Box<Error>> {
    let secret_key = utils::parse_secret_key(args, SECRET_KEY)?;
    let password = args.value_of(PASSWORD).map(|s| s.to_string());

    let credentials = Credentials::get(secret_key, password);

    let gas = value_t!(args, GAS, u32)?;
    let account: Address = utils::parse_hex_opt(args, ACCOUNT)?.parse()?;

    let contract_address: Address = parse_contract_address(args)?;

    let eth_url = parse_eth_url(args)?;

    return Ok(EthereumArgs {
        credentials,
        gas,
        account,
        contract_address,
        eth_url,
    });
}
