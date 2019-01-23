use crate::credentials::Credentials;
use crate::utils;
use clap::value_t;
use clap::Arg;
use clap::ArgMatches;
use failure::Error;
use failure::ResultExt;
use web3::types::Address;
use web3::types::H256;

const PASSWORD: &str = "password";
const SECRET_KEY: &str = "secret_key";
const GAS: &str = "gas";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const TENDERMINT_KEY: &str = "tendermint_key";
const BASE64_TENDERMINT_KEY: &str = "base64_tendermint_key";

#[derive(Debug, Clone)]
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

pub fn tendermint_key<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(TENDERMINT_KEY)
        .required(true)
        .takes_value(true)
        .help("public key of tendermint node")
}

pub fn base64_tendermint_key<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(BASE64_TENDERMINT_KEY)
        .long(BASE64_TENDERMINT_KEY)
        .help("allows to use base64 tendermint key")
}

pub fn with_ethereum_args<'a, 'b>(args: &[Arg<'a, 'b>]) -> Vec<Arg<'a, 'b>> {
    // find last positional argument index, to add ethereum arguments after it
    let last_index = args.iter().filter_map(|a| a.index).max().unwrap_or(0);

    let mut eth_args = vec![
        contract_address().index(last_index + 1),
        Arg::with_name(ACCOUNT)
            .required(true)
            .index(last_index + 2)
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
    ];

    // append args
    eth_args.extend_from_slice(args);

    // sort so positional arguments are always at the end, to reverse them later (clap nuance)
    eth_args.sort_unstable_by_key(|a| a.index);

    // reverse so positional arguments are always at the beginning (clap nuance)
    eth_args.reverse();

    eth_args
}

pub fn parse_contract_address(args: &ArgMatches) -> Result<Address, Error> {
    Ok(utils::parse_hex_opt(args, CONTRACT_ADDRESS)?.parse::<Address>()?)
}

pub fn parse_eth_url(args: &ArgMatches) -> Result<String, clap::Error> {
    value_t!(args, ETH_URL, String)
}

pub fn parse_ethereum_args(args: &ArgMatches) -> Result<EthereumArgs, Error> {
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

pub fn parse_tendermint_key(args: &ArgMatches) -> Result<H256, Error> {
    let tendermint_key = utils::parse_hex_opt(args, TENDERMINT_KEY)?.to_owned();
    let tendermint_key = if args.is_present(BASE64_TENDERMINT_KEY) {
        let arr = base64::decode(&tendermint_key)?;
        hex::encode(arr)
    } else {
        tendermint_key
    };

    let tendermint_key: H256 = tendermint_key
        .parse::<H256>()
        .context("error parsing tendermint key")?;

    Ok(tendermint_key)
}
