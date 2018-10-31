extern crate clap;
extern crate indicatif;
extern crate reqwest;
extern crate web3;
extern crate console;

use std::fs::File;
use std::error::Error;
use clap::{Arg, App};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Url, UrlError, Client};
use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, H256};
use console::style;

fn main() {
    let matches = App::new(format!("{}", style("Fluence Code Publisher").blue().bold()))
        .version("0.1.0")
        .author(format!("{}", style("Fluence Labs").blue().bold()).as_str())
        .about(format!("{}", style("Console utility for deploying code to fluence cluster").blue().bold()).as_str())
        .arg(Arg::with_name("path")
            .required(true)
            .takes_value(true)
            .index(1)
            .help("path to compiled `wasm` code"))
        .arg(Arg::with_name("account").alias("account")
            .required(true).alias("account").long("account").short("a")
            .takes_value(true)
            .help("ethereum account without `0x`"))
        .arg(Arg::with_name("contract_address").alias("contract_address")
            .required(true)
            .takes_value(true)
            .index(2)
            .help("deployer contract address without `0x`"))
        .arg(Arg::with_name("swarm_url").alias("swarm_url").long("swarm_url").short("s")
            .required(false)
            .takes_value(true)
            .help("http address to swarm node")
            .default_value("http://localhost:8500/")) //todo: use public gateway
        .arg(Arg::with_name("eth_url").alias("eth_url").long("eth_url").short("e")
            .required(false)
            .takes_value(true)
            .help("http address to ethereum node")
            .default_value("http://localhost:8545/")) //todo: use public node or add light client
        .arg(Arg::with_name("password").alias("password").long("password").short("p")
            .required(false)
            .takes_value(true)
            .help("password to unlock account in ethereum client"))
        .get_matches();

    let path = matches.value_of("path").expect("Path must be specified.");

    let contract_address = matches.value_of("contract_address").expect("Path must be specified.");
    let contract_address: Address = contract_address.parse().unwrap();

    let account = matches.value_of("account").expect("Account must be specified.");
    let account: Address = account.parse().unwrap();

    let swarm_url = matches.value_of("swarm_url").unwrap();
    let eth_url = matches.value_of("eth_url").unwrap();

    let password = matches.value_of("password");

    // uploading code to swarm
    let bar = create_progress_bar("1/2","Сode loading...");
    let hash = upload(swarm_url, path).unwrap();
    bar.finish_with_message("Code uploaded.");

    // sending transaction with the hash of file with code to ethereum
    let bar = create_progress_bar("2/2", "Submitting code to the smart contract...");
    let transaction = publish_to_contract(account, contract_address, hash, password, eth_url);
    bar.finish_with_message("Code submitted.");

    let formatted_finish_msg = style("Code published. Submitted transaction").blue();
    let formatted_tx = style(transaction.unwrap()).red().bold();

    println!("{}: {:?}", formatted_finish_msg, formatted_tx);
}

/// Publishes hash of the code (address in swarm) to the `Deployer` smart contract
fn publish_to_contract(account: Address, contract_address: Address, hash: String,
                       password: Option<&str>, eth_url: &str) -> Result<H256, Box<Error>> {

    let hash: H256 = hash.parse().unwrap();

    let (_eloop, transport) = web3::transports::Http::new(eth_url)?;
    let web3 = web3::Web3::new(transport);

    match password {
        Some(p) => {
            web3.personal().unlock_account(account, p, None).wait()?;
        }
        _ => {}
    }

    let json = include_bytes!("../Deployer.abi");

    let contract = Contract::from_json(web3.eth(), contract_address, json)?;

    //todo: add correct receipts
    let receipt: H256 = "0000000000000000000000000000000000000000000000000000000000000000".parse()?;
    let result_code_publish = contract.call("addCode", (hash, receipt, 3), account, Options::default());
    Ok(result_code_publish.wait()?)
}

/// Parses URL from string
fn parse_url(url: &str) -> Result<Url, UrlError> {
    match Url::parse(url) {
        Ok(url) => Ok(url),
        Err(error) if error == UrlError::RelativeUrlWithoutBase => {
            let url_with_base = format!("{}{}", "http://", url);
            Url::parse(url_with_base.as_str())
        }
        Err(error) => Err(error),
    }
}

/// Uploads file from path to the swarm
fn upload(url: &str, path: &str) -> Result<String, Box<Error>> {
    let file = File::open(path).expect("file not found");
    let mut url = parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client.post(url)
        .body(file)
        .header("Content-Type", "application/octet-stream")
        .send().and_then(|mut r| {
        r.text()
    })?;

    Ok(res)
}

const TEMPLATE: &str = "[{prefix:.blue}] {spinner} {msg:.blue} ---> [{elapsed_precise:.blue}]";

/// Creates a spinner progress bar, that will be tick at once
fn create_progress_bar(prefix: &str, msg: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_message(msg);
    bar.set_prefix(prefix);
    bar.enable_steady_tick(100);
    bar.set_style(ProgressStyle::default_spinner().template(TEMPLATE));

    bar
}
