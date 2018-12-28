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
use contract_func::ContractCaller;
use credentials::Credentials;
use ethkey::Secret;
use reqwest::Client;
use std::boxed::Box;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use utils;
use web3::types::{Address, H256};

const PATH: &str = "path";
const ACCOUNT: &str = "account";
const CONTRACT_ADDRESS: &str = "contract_address";
const ETH_URL: &str = "eth_url";
const PASSWORD: &str = "password";
const CLUSTER_SIZE: &str = "cluster_size";
const SWARM_URL: &str = "swarm_url";
const SECRET_KEY: &str = "secret_key";
const GAS: &str = "gas";

#[derive(Debug)]
pub struct Publisher {
    bytes: Vec<u8>,
    contract_address: Address,
    account: Address,
    swarm_url: String,
    eth_url: String,
    credentials: Credentials,
    cluster_size: u8,
    gas: u32,
}

impl Publisher {
    /// Creates `Publisher` structure
    pub fn new(
        bytes: Vec<u8>,
        contract_address: Address,
        account: Address,
        swarm_url: String,
        eth_url: String,
        credentials: Credentials,
        cluster_size: u8,
        gas: u32,
    ) -> Publisher {
        Publisher {
            bytes,
            contract_address,
            account,
            swarm_url,
            eth_url,
            credentials,
            cluster_size,
            gas,
        }
    }

    /// Sends code to Swarm and publishes the hash of the file from Swarm to Fluence smart contract
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
            //todo: add correct receipts
            let receipt: H256 =
                "0000000000000000000000000000000000000000000000000000000000000000".parse()?;

            let contract = ContractCaller::new(self.contract_address, &self.eth_url)?;

            let pin_to_nodes: Vec<H256> = [].to_vec();

            contract.call_contract(
                self.account,
                &self.credentials,
                "addApp",
                (hash, receipt, u64::from(self.cluster_size), pin_to_nodes),
                self.gas,
            )
        };

        // sending transaction with the hash of file with code to ethereum
        if show_progress {
            utils::with_progress(
                "Submitting the code to the smart contract...",
                "2/2",
                "Code submitted.",
                publish_to_contract_fn,
            )
        } else {
            publish_to_contract_fn()
        }
    }
}

/// Creates `Publisher` from arguments
pub fn parse(matches: &ArgMatches) -> Result<Publisher, Box<Error>> {
    let path = matches.value_of(PATH).unwrap().to_string();

    let contract_address = matches
        .value_of(CONTRACT_ADDRESS)
        .unwrap()
        .trim_left_matches("0x");
    let contract_address: Address = contract_address.parse()?;

    let account = matches.value_of(ACCOUNT).unwrap().trim_left_matches("0x");
    let account: Address = account.parse()?;

    let swarm_url = matches.value_of(SWARM_URL).unwrap().to_string();
    let eth_url = matches.value_of(ETH_URL).unwrap().to_string();

    let secret_key = matches
        .value_of(SECRET_KEY)
        .map(|s| Secret::from_str(s.trim_left_matches("0x")).unwrap());
    let password = matches.value_of(PASSWORD).map(|s| s.to_string());

    let credentials = Credentials::get(secret_key, password);

    let cluster_size: u8 = matches.value_of(CLUSTER_SIZE).unwrap().parse()?;

    let mut file = File::open(path)?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let gas: u32 = matches.value_of(GAS).unwrap().parse()?;

    Ok(Publisher::new(
        buf.to_owned(),
        contract_address,
        account,
        swarm_url,
        eth_url,
        credentials,
        cluster_size,
        gas,
    ))
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("publish")
        .about("Publish code to ethereum blockchain")
        .args(&[
            Arg::with_name(PATH)
                .required(true)
                .takes_value(true)
                .index(1)
                .help("path to compiled `wasm` code"),
            Arg::with_name(CONTRACT_ADDRESS)
                .alias(CONTRACT_ADDRESS)
                .required(true)
                .takes_value(true)
                .index(2)
                .help("fluence contract address"),
            Arg::with_name(ACCOUNT)
                .alias(ACCOUNT)
                .required(true)
                .index(3)
                .takes_value(true)
                .help("ethereum account"),
            Arg::with_name(SWARM_URL)
                .alias(SWARM_URL)
                .long(SWARM_URL)
                .short("w")
                .required(false)
                .takes_value(true)
                .help("http address to swarm node")
                .default_value("http://localhost:8500/"),
            //todo: use public gateway
            Arg::with_name(ETH_URL)
                .alias(ETH_URL)
                .long(ETH_URL)
                .short("e")
                .required(false)
                .takes_value(true)
                .help("http address to ethereum node")
                .default_value("http://localhost:8545/"),
            //todo: use public node or add light client
            Arg::with_name(PASSWORD)
                .alias(PASSWORD)
                .long(PASSWORD)
                .short("p")
                .required(false)
                .takes_value(true)
                .help("password to unlock account in ethereum client"),
            Arg::with_name(SECRET_KEY)
                .alias(SECRET_KEY)
                .long(SECRET_KEY)
                .short("s")
                .required(false)
                .takes_value(true)
                .help("the secret key to sign transactions"),
            Arg::with_name(CLUSTER_SIZE)
                .alias(CLUSTER_SIZE)
                .long(CLUSTER_SIZE)
                .short("cs")
                .required(false)
                .takes_value(true)
                .default_value("3")
                .help("cluster's size that needed to deploy this code"),
            Arg::with_name(GAS)
                .alias(GAS)
                .long(GAS)
                .short("g")
                .required(false)
                .takes_value(true)
                .default_value("1000000")
                .help("maximum gas to spend"),
        ])
}

/// Uploads bytes of code to the Swarm
fn upload_code_to_swarm(url: &str, bytes: &[u8]) -> Result<String, Box<Error>> {
    let mut url = utils::parse_url(url)?;
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
    use credentials::Credentials;
    use ethkey::Secret;
    use publisher::Publisher;
    use std::error::Error;
    use web3;
    use web3::futures::Future;
    use web3::types::*;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

    fn generate_publisher(account: &str, creds: Credentials) -> Publisher {
        let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap();

        let bytes = vec![1, 2, 3];

        Publisher::new(
            bytes,
            contract_address,
            account.parse().unwrap(),
            String::from("http://localhost:8500"),
            String::from("http://localhost:8545/"),
            creds,
            5,
            1000000,
        )
    }

    pub fn generate_with<F>(account: &str, func: F) -> Publisher
    where
        F: FnOnce(&mut Publisher),
    {
        let mut publisher = generate_publisher(account, Credentials::No);
        func(&mut publisher);
        publisher
    }

    pub fn generate_new_account(with_pass: bool) -> Publisher {
        generate_with(OWNER, |p| {
            let (_eloop, transport) = web3::transports::Http::new(&p.eth_url).unwrap();
            let web3 = web3::Web3::new(transport);
            let acc = web3.personal().new_account("123").wait().unwrap();
            p.account = acc;

            if with_pass {
                p.credentials = Credentials::Password(String::from("123"));
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
        let publisher = generate_with("02f906f8b3b932fd282109a5b8dc732ba2329888", |p| {
            p.swarm_url = String::from("http://123.5.6.7:8385");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_eth_url() -> Result<(), Box<Error>> {
        let publisher = generate_with("fa0de43c68bea2167181cd8a83f990d02a049336", |p| {
            p.eth_url = String::from("http://117.2.6.7:4476");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_out_of_gas() -> Result<(), Box<Error>> {
        let publisher = generate_with("fa0de43c68bea2167181cd8a83f990d02a049336", |p| {
            p.gas = 1;
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_to_contract_success() -> Result<(), Box<Error>> {
        let publisher =
            generate_publisher("64b8f12d14925394ae0119466dff6ff2b021a3e9", Credentials::No);

        publisher.publish(false)?;

        Ok(())
    }

    #[test]
    fn publish_to_contract_with_secret_success() -> Result<(), Box<Error>> {
        let secret_arr: H256 =
            "647334ad14cda7f79fecdf2b9e0bb2a0904856c36f175f97c83db181c1060414".parse()?;
        let secret = Secret::from(secret_arr);
        let publisher = generate_publisher(
            "ee75d7d2f7286dfa893f7ff58323917902889afe",
            Credentials::Secret(secret),
        );

        publisher.publish(false)?;

        Ok(())
    }
}
