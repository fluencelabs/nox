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
use publisher::app::Publisher;
use reqwest::{Client, Url, UrlError};
use std::error::Error;
use web3::contract::{Contract, Options};
use web3::futures::Future;
use web3::types::{Address, H256, U256};
use web3::{Transport, Web3};

fn main() {
    let publisher = app::init().unwrap();

    let transaction = publish(publisher, true);

    let formatted_finish_msg = style("Code published. Submitted transaction").blue();
    let formatted_tx = style(transaction.unwrap()).red().bold();

    println!("{}: {:?}", formatted_finish_msg, formatted_tx);
}

/// Creates progress bar in the console until the work is over
///
/// # Arguments
///
/// * `msg` - message on progress bar while working in progress
/// * `prefix`
/// * `finish` - message after work is done
/// * `work` - some function to be done
///
/// # Examples
/// ```
/// with_progress("Code uploading...", "1/2", "Code uploaded.", upload_fn)
/// ```
/// The output while processing:
/// ```
/// [1/2] ⠙ Code uploading... ---> [00:00:05]
/// ```
/// The output on the finish:
/// ```
/// [1/2]   Code uploaded. ---> [00:00:10]
/// ```
///
pub fn with_progress<U, F: FnOnce() -> U>(msg: &str, prefix: &str, finish: &str, work: F) -> U {
    let bar = create_progress_bar(prefix, msg);
    let result = work();
    bar.finish_with_message(finish);
    result
}

fn publish(publisher: Publisher, show_progress: bool) -> Result<H256, Box<Error>> {

    let upload_to_swarm_fn = || -> Result<H256, Box<Error>> {
        let hash = upload_code_to_swarm(&publisher.swarm_url, &publisher.bytes)?;
        let hash = hash.parse()?;
        Ok(hash)
    };

    let hash: H256 = if show_progress {
        with_progress("Code uploading...", "1/2", "Code uploaded.", upload_to_swarm_fn)
    } else {
        upload_to_swarm_fn()
    }?;

    let publish_to_contract_fn = || -> Result<H256, Box<Error>> {
        let pass = publisher.password.as_ref().map(|s| s.as_str());
        publish_to_contract(
            publisher.account,
            publisher.contract_address,
            hash,
            pass,
            &publisher.eth_url,
            publisher.cluster_size,
        )
    };

    // sending transaction with the hash of file with code to ethereum
    let transaction = if show_progress {
        with_progress(
            "Submitting the code to the smart contract...",
            "2/2",
            "Code submitted.",
            publish_to_contract_fn,
        )
    } else {
        publish_to_contract_fn()
    };
    transaction
}

fn get_contract<T: Transport>(
    web3: Web3<T>,
    contract_address: Address,
) -> Result<Contract<T>, Box<Error>> {
    let json = include_bytes!("../../bootstrap/contracts/compiled/Deployer.abi");

    Ok(Contract::from_json(web3.eth(), contract_address, json)?)
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
            let gl: U256 = 100_000.into();
            o.gas = Some(gl);
        }),
    );
    Ok(result_code_publish.wait()?)
}

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

fn upload_code_to_swarm(url: &str, bytes: &Vec<u8>) -> Result<String, Box<Error>> {
    let mut url = parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client
        .post(url)
        .body(bytes.to_vec())
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

#[cfg(test)]
mod tests {
    use super::{get_contract, publish};
    use publisher::app::Publisher;
    use std::error::Error;
    use web3;
    use web3::contract::Options;
    use web3::futures::Future;
    use web3::types::*;

    #[cfg(test)]
    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

    #[cfg(test)]
    fn generate_publisher() -> Publisher {
        let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap();

        let bytes = vec![1, 2, 3];

        Publisher::new(
            bytes,
            contract_address,
            OWNER.parse().unwrap(),
            String::from("http://localhost:8500"),
            String::from("http://localhost:8545/"),
            None,
            5,
        )
    }

    #[cfg(test)]
    pub fn generate_with<F>(func: F) -> Publisher
    where
        F: FnOnce(&mut Publisher),
    {
        let mut publisher = generate_publisher();
        func(&mut publisher);
        publisher
    }

    #[cfg(test)]
    pub fn generate_with_account(account: Address) -> Publisher {
        generate_with(|p| {
            p.account = account;
        })
    }

    #[cfg(test)]
    pub fn generate_new_account(with_pass: bool) -> Publisher {
        generate_with(|p| {
            let (_eloop, transport) = web3::transports::Http::new(&p.eth_url).unwrap();
            let web3 = web3::Web3::new(transport);
            let acc = web3.personal().new_account("123").wait().unwrap();
            p.account = acc;
            if with_pass {
                p.password = Some(String::from("123"));
            }
        })
    }

    #[cfg(test)]
    pub fn add_to_white_list(
        eth_url: &str,
        account: Address,
        contract_address: Address,
    ) -> Result<H256, Box<Error>> {
        let (_eloop, transport) = web3::transports::Http::new(eth_url)?;
        let web3 = web3::Web3::new(transport);

        let contract = get_contract(web3, contract_address)?;

        Ok(contract
            .call(
                "addAddressToWhitelist",
                account,
                OWNER.parse()?,
                Options::default(),
            )
            .wait()?)
    }

    #[test]
    fn publish_wrong_password() -> Result<(), Box<Error>> {
        let publisher = generate_new_account(false);

        let result = publish(publisher, false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_no_eth() -> Result<(), Box<Error>> {
        let publisher = generate_new_account(true);

        let result = publish(publisher, false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_swarm_url() -> Result<(), Box<Error>> {
        let publisher = generate_with(|p| {
            p.swarm_url = String::from("http://127.0.6.7:8545");
        });

        let result = publish(publisher, false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_eth_url() -> Result<(), Box<Error>> {
        let publisher = generate_with(|p| {
            p.eth_url = String::from("http://127.0.6.7:8545");
        });

        let result = publish(publisher, false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_to_contract_without_whitelist() -> Result<(), Box<Error>> {
        let publisher = generate_with_account("fa0de43c68bea2167181cd8a83f990d02a049336".parse()?);

        let result = publish(publisher, false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_to_contract_success() -> Result<(), Box<Error>> {
        let publisher = generate_with_account("02f906f8b3b932fd282109a5b8dc732ba2329888".parse()?);

        add_to_white_list(
            &publisher.eth_url,
            publisher.account,
            publisher.contract_address,
        )?;

        publish(publisher, false)?;

        Ok(())
    }
}
