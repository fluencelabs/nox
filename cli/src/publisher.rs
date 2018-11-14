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
extern crate web3;

use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use reqwest::{Client, Url, UrlError};
use std::boxed::Box;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use utils;
use web3::contract::Options;
use web3::types::{Address, H256, U256};

pub struct Publisher {
    pub bytes: Vec<u8>,
    pub contract_address: Address,
    pub account: Address,
    pub swarm_url: String,
    pub eth_url: String,
    pub password: Option<String>,
    pub cluster_size: u8,
}

impl Publisher {
    pub fn new(
        bytes: Vec<u8>,
        contract_address: Address,
        account: Address,
        swarm_url: String,
        eth_url: String,
        password: Option<String>,
        cluster_size: u8,
    ) -> Publisher {
        Publisher {
            bytes,
            contract_address,
            account,
            swarm_url,
            eth_url,
            password,
            cluster_size,
        }
    }

    pub fn publish(&self, show_progress: bool) -> Result<H256, Box<Error>> {
        let upload_to_swarm_fn = || -> Result<H256, Box<Error>> {
            let hash = upload_code_to_swarm(&self.swarm_url, &self.bytes)?;
            let hash = hash.parse()?;
            Ok(hash)
        };

        let hash: H256 = if show_progress {
            utils::with_progress(
                "Code uploading...",
                "1/2",
                "Code uploaded.",
                upload_to_swarm_fn,
            )
        } else {
            upload_to_swarm_fn()
        }?;

        let publish_to_contract_fn = || -> Result<H256, Box<Error>> {
            let pass = self.password.as_ref().map(|s| s.as_str());

            //todo: add correct receipts
            let receipt: H256 =
                "0000000000000000000000000000000000000000000000000000000000000000".parse()?;

            let options = Options::with(|o| {
                let gl: U256 = 100_000.into();
                o.gas = Some(gl);
            });

            utils::publish_to_contract(
                self.account,
                self.contract_address,
                pass,
                &self.eth_url,
                "addCode",
                (hash, receipt, self.cluster_size as u64),
                options,
            )
        };

        // sending transaction with the hash of file with code to ethereum
        let transaction = if show_progress {
            utils::with_progress(
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
}

pub fn parse(matches: ArgMatches) -> Result<Publisher, Box<std::error::Error>> {
    let path = matches.value_of("path").unwrap().to_string();

    let contract_address = matches
        .value_of("contract_address")
        .unwrap()
        .trim_left_matches("0x");
    let contract_address: Address = contract_address.parse()?;

    let account = matches.value_of("account").unwrap().trim_left_matches("0x");
    let account: Address = account.parse()?;

    let swarm_url = matches.value_of("swarm_url").unwrap().to_string();
    let eth_url = matches.value_of("eth_url").unwrap().to_string();

    let password = matches.value_of("password").map(|s| s.to_string());

    let cluster_size: u8 = matches.value_of("cluster_size").unwrap().parse()?;

    let mut file = File::open(path)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    Ok(Publisher::new(
        buf.to_owned(),
        contract_address,
        account,
        swarm_url,
        eth_url,
        password,
        cluster_size,
    ))
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("publish")
        .about("Publish code to ethereum blockchain.")
        .args(&[
            Arg::with_name("path")
                .required(true)
                .takes_value(true)
                .index(1)
                .help("path to compiled `wasm` code"),
            Arg::with_name("account")
                .alias("account")
                .required(true)
                .alias("account")
                .long("account")
                .short("a")
                .takes_value(true)
                .help("ethereum account"),
            Arg::with_name("contract_address")
                .alias("contract_address")
                .required(true)
                .takes_value(true)
                .index(2)
                .help("deployer contract address"),
            Arg::with_name("swarm_url")
                .alias("swarm_url")
                .long("swarm_url")
                .short("s")
                .required(false)
                .takes_value(true)
                .help("http address to swarm node")
                .default_value("http://localhost:8500/"),
            //todo: use public gateway
            Arg::with_name("eth_url")
                .alias("eth_url")
                .long("eth_url")
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
            //todo: use public node or add light client
            Arg::with_name("password")
                .alias("password")
                .long("password")
                .short("p")
                .required(false)
                .takes_value(true)
                .help("password to unlock account in ethereum client"),
            Arg::with_name("cluster_size")
                .alias("cluster_size")
                .long("cluster_size")
                .short("cs")
                .required(false)
                .takes_value(true)
                .default_value("3")
                .help("cluster's size that needed to deploy this code"),
        ])
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

#[cfg(test)]
mod tests {
    use publisher::Publisher;
    use std::error::Error;
    use utils;
    use web3;
    use web3::futures::Future;
    use web3::types::*;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

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

    pub fn generate_with<F>(func: F) -> Publisher
    where
        F: FnOnce(&mut Publisher),
    {
        let mut publisher = generate_publisher();
        func(&mut publisher);
        publisher
    }

    pub fn generate_with_account(account: Address) -> Publisher {
        generate_with(|p| {
            p.account = account;
        })
    }

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

    #[test]
    fn publish_wrong_password() -> Result<(), Box<Error>> {
        let publisher = generate_new_account(false);

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_no_eth() -> Result<(), Box<Error>> {
        let publisher = generate_new_account(true);

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_swarm_url() -> Result<(), Box<Error>> {
        let publisher = generate_with(|p| {
            p.swarm_url = String::from("http://127.0.6.7:8545");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_eth_url() -> Result<(), Box<Error>> {
        let publisher = generate_with(|p| {
            p.eth_url = String::from("http://127.0.6.7:8545");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_to_contract_without_whitelist() -> Result<(), Box<Error>> {
        let publisher = generate_with_account("fa0de43c68bea2167181cd8a83f990d02a049336".parse()?);

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_to_contract_success() -> Result<(), Box<Error>> {
        let publisher = generate_with_account("02f906f8b3b932fd282109a5b8dc732ba2329888".parse()?);

        utils::add_to_white_list(
            &publisher.eth_url,
            publisher.account,
            publisher.contract_address,
            OWNER.parse()?,
        )?;

        publisher.publish(false)?;

        Ok(())
    }
}
