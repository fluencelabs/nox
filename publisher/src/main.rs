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
extern crate console;
extern crate indicatif;
extern crate reqwest;
extern crate web3;

mod publisher;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use publisher::app;
use reqwest::{Client, Url, UrlError};
use std::error::Error;
use std::fs::File;
use web3::contract::{Contract, Options};
use web3::futures::Future;
use web3::types::{Address, H256};

fn main() {
    let publisher = app::init().unwrap();

    // uploading code to swarm
    let bar = create_progress_bar("1/2", "Code uploading...");
    let hash = upload(&publisher.swarm_url, &publisher.path).unwrap();
    bar.finish_with_message("Code uploaded.");

    // sending transaction with the hash of file with code to ethereum
    let bar = create_progress_bar("2/2", "Submitting the code to the smart contract...");
    let pass = publisher.password.as_ref().map(|s| s.as_str());
    let transaction = publish_to_contract(
        publisher.account,
        publisher.contract_address,
        &hash,
        pass,
        &publisher.eth_url,
        publisher.cluster_size,
    );
    bar.finish_with_message("Code submitted.");

    let formatted_finish_msg = style("Code published. Submitted transaction").blue();
    let formatted_tx = style(transaction.unwrap()).red().bold();

    println!("{}: {:?}", formatted_finish_msg, formatted_tx);
}

/// Publishes hash of the code (address in swarm) to the `Deployer` smart contract
fn publish_to_contract(
    account: Address,
    contract_address: Address,
    hash: &str,
    password: Option<&str>,
    eth_url: &str,
    cluster_size: u64,
) -> Result<H256, Box<Error>> {
    let hash: H256 = hash.parse().unwrap();

    let (_eloop, transport) = web3::transports::Http::new(&eth_url)?;
    let web3 = web3::Web3::new(transport);

    if let Some(p) = password { web3.personal().unlock_account(account, p, None).wait()?; }

    let json = include_bytes!("../Deployer.abi");

    let contract = Contract::from_json(web3.eth(), contract_address, json)?;

    //todo: add correct receipts
    let receipt: H256 =
        "0000000000000000000000000000000000000000000000000000000000000000".parse()?;
    let result_code_publish = contract.call(
        "addCode",
        (hash, receipt, cluster_size),
        account,
        Options::default(),
    );
    Ok(result_code_publish.wait()?)
}

/// Parses URL from string
fn parse_url(url: &str) -> Result<Url, UrlError> {
    match Url::parse(url) {
        Ok(url) => Ok(url),
        Err(error) if error == UrlError::RelativeUrlWithoutBase => {
            let url_with_base = format!("http://{}", url);
            Url::parse(url_with_base.as_str())
        }
        Err(error) => Err(error),
    }
}

/// Uploads file from path to the swarm
fn upload(url: &str, path: &str) -> Result<String, Box<Error>> {
    let file = File::open(path)?;
    let mut url = parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client
        .post(url)
        .body(file)
        .header("Content-Type", "application/octet-stream")
        .send()
        .and_then(|mut r| r.text())?;

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
