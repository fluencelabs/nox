extern crate clap;
extern crate indicatif;
extern crate reqwest;
extern crate web3;
extern crate ethabi;

use std::fs::File;
use std::thread;
use std::error::Error;
use clap::{Arg, App, ArgSettings};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Url, UrlError, Client};
use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, H160, H256};

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
        .arg(Arg::with_name("contract_address").alias("contract_address").long("contract_address").short("c")
            .required(false)
            .takes_value(true)
            .help("deployer contract address"))
        .arg(Arg::with_name("swarm_node").alias("swarm_node").long("swarm_node").short("s")
            .required(false)
            .takes_value(true)
            .help("http address to swarm node")
            .default_value("http://localhost:8500/"))
        .arg(Arg::with_name("password").alias("password").long("password").short("p")
            .required(false)
            .takes_value(true)
            .help("password to unlock account in ethereum client"))
        .get_matches();

    let path = matches.value_of("path").expect("Path must be specified.");
    println!("{}", path);

    let config = matches.value_of("config");
    println!("{}", config.unwrap_or("none"));

    let contract_address = matches.value_of("contract_address");
    println!("{}", contract_address.unwrap_or("none"));

    let swarm_node = matches.value_of("swarm_node").unwrap();
    println!("{}", swarm_node);

    let mut bar = create_progress_bar(false, "Сode loading...", None);

    let file = File::open(path).expect("file not found");
    let hash = upload(swarm_node, file).unwrap();
    bar.finish_with_message("Code has been uploaded\n");

    println!("{}", hash);


    let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);
    let accounts = web3.eth().accounts().wait().unwrap();

    println!("Accounts: {:?}", accounts);

    let acc: Address = "7aD615F735528C8c975777e83befFe8042Ad8Ffb".parse().unwrap();

    let password = matches.value_of("password");

    match password {
        Some(p) => {
            web3.personal().unlock_account(acc, p, None).wait().unwrap();
        }
        _ => {}
    }

    let addr: Address = "Dc596f73cDDd26d138fc179Eb4525AEdAF078D79".parse().unwrap();

    let json = include_bytes!("../Deployer.abi");

    let contract = Contract::from_json(web3.eth(), addr, json).unwrap();

    let hash: H256 = hash.parse().unwrap();
    let receipt: H256 = "0000000000000000000000000000000000000000000000000000000000000000".parse().unwrap();
    let result_code_publish = contract.call("addCode", (hash, receipt, 3), acc, Options::default());
    let code_published = result_code_publish.wait().unwrap();

    println!("Result: {:?}", code_published);
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
    let mut res = client.post(url)
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
            bar.set_style(ProgressStyle::default_spinner().template("{msg}{spinner}"));

        }

    };

    bar
}
