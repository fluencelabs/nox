extern crate clap;
extern crate indicatif;
extern crate reqwest;
extern crate web3;
extern crate ethabi;

use std::fs::File;
use clap::{Arg, App};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Url, UrlError, Client};
use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, H256};

fn main() {
    let matches = App::new("Fluence")
        .version("0.1.0")
        .author("Fluence Labs")
        .about("Console utility for deploying code to fluence cluster")
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
            .default_value("http://localhost:8500/"))
        .arg(Arg::with_name("eth_url").alias("eth_url").long("eth_url").short("e")
            .required(false)
            .takes_value(true)
            .help("http address to ethereum node")
            .default_value("http://localhost:8545/"))
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

    let password = matches.value_of("password");

    let bar = create_progress_bar(false, "Сode loading...", None);

    let file = File::open(path).expect("file not found");
    let hash = upload(swarm_url, file).unwrap();
    bar.finish_with_message("Code has been uploaded.");

    let bar = create_progress_bar(false, "Submitting code to the smart contract...", None);

    let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);

    match password {
        Some(p) => {
            web3.personal().unlock_account(account, p, None).wait().unwrap();
        }
        _ => {}
    }

    let json = include_bytes!("../Deployer.abi");

    let contract = Contract::from_json(web3.eth(), contract_address, json).unwrap();

    let hash: H256 = hash.parse().unwrap();
    let receipt: H256 = "0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap();
    let result_code_publish = contract.call("addCode", (hash, receipt, 3), account, Options::default());
    let code_published = result_code_publish.wait().unwrap();

    bar.finish_with_message("Code submitted.");

    println!("Code published. Submitted transaction: {:?}", code_published);
}

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

fn upload(url: &str, file: File) -> Result<String, Box<std::error::Error>> {
    let mut url = parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client.post(url)
        .body(file)
        .header("Content-Type", "application/octet-stream")
        .send().and_then(|mut r| {
        r.text()
    })?;

    let sleep_time = std::time::Duration::from_millis(1000);

    std::thread::sleep(sleep_time);

    Ok(res)
}

fn create_progress_bar(quiet_mode: bool, msg: &str, length: Option<u64>) -> ProgressBar {
    let bar = match quiet_mode {
        true => ProgressBar::hidden(),
        false => {
            match length {
                Some(len) => ProgressBar::new(len),
                None => ProgressBar::new_spinner(),
            }
        }
    };

    bar.set_message(msg);
    match length.is_some() {
        true => bar
            .set_style(ProgressStyle::default_bar()
                .template("{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}")
                .progress_chars("=> ")),
        false => {
            bar.enable_steady_tick(100);
            bar.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {spinner:.green} {spinner:.green} {msg} -----> [{elapsed_precise}]"));

        }

    };

    bar
}
