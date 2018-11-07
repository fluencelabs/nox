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
use web3::types::{Address, H256, U256};
use web3::{Transport, Web3};

fn main() {
    let publisher = app::init().unwrap();

    // uploading code to swarm
    let bar = create_progress_bar("1/2", "Code uploading...");
    let hash = upload(&publisher.swarm_url, &publisher.path).unwrap();
    let hash: H256 = hash.parse().unwrap();
    bar.finish_with_message("Code uploaded.");

    // sending transaction with the hash of file with code to ethereum
    let bar = create_progress_bar("2/2", "Submitting the code to the smart contract...");
    let pass = publisher.password.as_ref().map(|s| s.as_str());
    let transaction = publish_to_contract(
        publisher.account,
        publisher.contract_address,
        hash,
        pass,
        &publisher.eth_url,
        publisher.cluster_size,
    );
    bar.finish_with_message("Code submitted.");

    let formatted_finish_msg = style("Code published. Submitted transaction").blue();
    let formatted_tx = style(transaction.unwrap()).red().bold();

    println!("{}: {:?}", formatted_finish_msg, formatted_tx);
}

fn get_contract<T: Transport>(
    web3: Web3<T>,
    contract_address: Address,
) -> Result<Contract<T>, Box<Error>> {
    let json = include_bytes!("../../bootstrap/contracts/compiled/Deployer.abi");

    Contract::from_json(web3.eth(), contract_address, json).map_err(|e| e.into())
}

/// Publishes hash of the code (address in swarm) to the `Deployer` smart contract
fn publish_to_contract(
    account: Address,
    contract_address: Address,
    hash: H256,
    password: Option<&str>,
    eth_url: &str,
    cluster_size: u64,
) -> Result<H256, Box<Error>> {
    let (_eloop, transport) = web3::transports::Http::new(&eth_url)?;
    let web3 = web3::Web3::new(transport);

    if let Some(p) = password {
        web3.personal().unlock_account(account, p, None).wait()?;
    }

    let contract = get_contract(web3, contract_address)?;

    //todo: add correct receipts
    let receipt: H256 =
        "0000000000000000000000000000000000000000000000000000000000000000".parse()?;
    let result_code_publish = contract.call(
        "addCode",
        (hash, receipt, cluster_size),
        account,
        Options::with(|o| {
            //todo specify
            let gp: U256 = 22_000_000_000i64.into();
            o.gas_price = Some(gp);

            let gl: U256 = 4_300_000.into();
            o.gas = Some(gl);
        }),
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

mod tests {
    use super::{get_contract, publish_to_contract};
    use std::error::Error;
    use std::process::Command;
    use web3;
    use web3::contract::Options;
    use web3::futures::Future;
    use web3::types::{Address, H256};

    fn run_ganache() -> Result<(), Box<Error>> {
        let mut install_cmd = Command::new("npm");
        install_cmd.current_dir("../bootstrap/");
        install_cmd.args(&["install"]);
        install_cmd.status().unwrap();

        let mut run_cmd = Command::new("npm");
        run_cmd.current_dir("../bootstrap/");
        run_cmd.args(&["run", "ganache"]);
        run_cmd.status().unwrap();

        let mut deploy_cmd = Command::new("npm");
        deploy_cmd.current_dir("../bootstrap/");
        deploy_cmd.args(&["run", "migrate"]);
        deploy_cmd.status().unwrap();

        Ok(())
    }

    #[test]
    fn publish_to_contract_success() -> Result<(), Box<Error>> {
        fn test() -> Result<H256, Box<Error>> {
            run_ganache()?;

            let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse()?;

            let account: Address = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d".parse()?;

            let hash = "d1f25a870a7bb7e5d526a7623338e4e9b8399e76df8b634020d11d969594f24a";
            let hash: H256 = hash.parse()?;

            let url = "http://127.0.0.1:8545";

            let (_eloop, transport) = web3::transports::Http::new(&url)?;
            let web3 = web3::Web3::new(transport);

            let contract = get_contract(web3, contract_address)?;

            contract
                .call(
                    "addAddressToWhitelist",
                    account,
                    account.to_owned(),
                    Options::default(),
                )
                .wait()?;

            publish_to_contract(account, contract_address, hash, None, url, 5)
        }

        let result = test();

        stop_ganache();

        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.unwrap(),
            "cc73807a63a8064a17b0fec48ffb828bc1ac330c2970d00dfbf88aebe263d876".parse()?
        );

        Ok(())
    }

    fn stop_ganache() {
        let mut stop_cmd = Command::new("pkill");
        stop_cmd.current_dir("../bootstrap/");
        stop_cmd.args(&["-f", "ganache"]);
        stop_cmd.status().unwrap();
    }
}
