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

use std::boxed::Box;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

use clap::ArgMatches;
use clap::{value_t, App, Arg, SubCommand};
use derive_getters::Getters;
use reqwest::Client;
use web3::types::H256;

use crate::command::{parse_ethereum_args, with_ethereum_args, EthereumArgs};
use crate::contract_func::contract::functions::add_app;
use crate::contract_func::ContractCaller;
use crate::utils;

const PATH: &str = "path";
const CLUSTER_SIZE: &str = "cluster_size";
const SWARM_URL: &str = "swarm_url";
const PINNED: &str = "pin_to";
const PIN_BASE64: &str = "base64";

#[derive(Debug, Getters)]
pub struct Publisher {
    bytes: Vec<u8>,
    swarm_url: String,
    cluster_size: u8,
    pin_to_nodes: Vec<H256>,
    eth: EthereumArgs,
}

impl Publisher {
    /// Creates `Publisher` structure
    pub fn new(
        bytes: Vec<u8>,
        swarm_url: String,
        cluster_size: u8,
        pin_to_nodes: Vec<H256>,
        eth: EthereumArgs,
    ) -> Publisher {
        Publisher {
            bytes,
            swarm_url,
            cluster_size,
            pin_to_nodes,
            eth,
        }
    }

    /// Sends code to Swarm and publishes the hash of the file from Swarm to Fluence smart contract
    pub fn publish(&self, show_progress: bool) -> Result<H256, Box<Error>> {
        let upload_to_swarm_fn = || -> Result<H256, Box<Error>> {
            let hash = upload_code_to_swarm(&self.swarm_url.as_str(), &self.bytes.as_slice())?;
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

            let contract =
                ContractCaller::new(self.eth.contract_address, self.eth.eth_url.as_str())?;

            let (call_data, _) = add_app::call(
                hash,
                receipt,
                u64::from(self.cluster_size),
                self.pin_to_nodes.clone(),
            );

            contract.call_contract(
                self.eth.account,
                &self.eth.credentials,
                call_data,
                self.eth.gas,
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

fn parse_pinned(args: &ArgMatches) -> Result<Vec<H256>, Box<Error>> {
    let pin_to_nodes = args.values_of(PINNED).unwrap_or_default();

    let pin_to_nodes = pin_to_nodes.into_iter();

    let pin_to_nodes: Result<Vec<H256>, Box<Error>> = pin_to_nodes
        .map(|node_id| {
            let node_id = if args.is_present(PIN_BASE64) {
                let arr = base64::decode(node_id)?;
                hex::encode(arr)
            } else {
                node_id.trim_start_matches("0x").into()
            };

            let node_id = node_id.parse::<H256>()?;
            Ok(node_id)
        })
        .collect();

    let pin_to_nodes = pin_to_nodes.map_err(|e| format!("unable to parse {}: {}", PINNED, e))?;

    Ok(pin_to_nodes)
}

/// Creates `Publisher` from arguments
pub fn parse(matches: &ArgMatches) -> Result<Publisher, Box<Error>> {
    let path = value_t!(matches, PATH, String)?; //TODO use is_file from clap_validators
    let mut file = File::open(path).map_err(|e| format!("can't open WASM file: {}", e))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let swarm_url = value_t!(matches, SWARM_URL, String)?;
    let cluster_size = value_t!(matches, CLUSTER_SIZE, u8)?;
    let eth = parse_ethereum_args(matches)?;

    let pin_to_nodes = parse_pinned(matches)?;
    if pin_to_nodes.len() > 0 && pin_to_nodes.len() > (cluster_size as usize) {
        return Err(format!(
            "number of pin_to nodes should be less or equal to the desired cluster_size"
        )
        .into());
    }

    Ok(Publisher::new(
        buf.to_owned(),
        swarm_url,
        cluster_size,
        pin_to_nodes,
        eth,
    ))
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let my_args = &[
        Arg::with_name(PATH)
            .required(true)
            .takes_value(true)
            .index(1)
            .help("path to compiled `wasm` code"),
        Arg::with_name(SWARM_URL)
            .long(SWARM_URL)
            .short("w")
            .required(false)
            .takes_value(true)
            .help("http address to swarm node")
            .default_value("http://localhost:8500/"),
        Arg::with_name(CLUSTER_SIZE)
            .long(CLUSTER_SIZE)
            .short("cs")
            .required(false)
            .takes_value(true)
            .default_value("3")
            .help("cluster's size that needed to deploy this code"),
        Arg::with_name(PINNED)
            .long(PINNED)
            .short("P")
            .required(false)
            .takes_value(true)
            .multiple(true)
            .value_name("<key>")
            .help(
                "Tendermint public keys of pinned workers for application (space-separated list)",
            ),
        Arg::with_name(PIN_BASE64)
            .long(PIN_BASE64)
            .required(false)
            .takes_value(false)
            .help("If specified, tendermint keys for pin_to flag treated as base64"),
    ];

    SubCommand::with_name("publish")
        .about("Publish code to ethereum blockchain")
        .args(with_ethereum_args(my_args).as_slice())
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
    use std::error::Error;

    use ethkey::Secret;
    use web3;
    use web3::futures::Future;
    use web3::types::H256;
    use web3::types::*;

    use crate::command::EthereumArgs;
    use crate::credentials::Credentials;
    use crate::publisher::Publisher;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

    fn generate_publisher(account: &str, creds: Credentials) -> Publisher {
        let contract_address: Address = "9995882876ae612bfd829498ccd73dd962ec950a".parse().unwrap();

        let bytes = vec![1, 2, 3];

        let eth = EthereumArgs {
            credentials: creds,
            gas: 1000000,
            account: account.parse().unwrap(),
            contract_address,
            eth_url: String::from("http://localhost:8545"),
        };

        Publisher::new(
            bytes,
            String::from("http://localhost:8500/"),
            5,
            vec![],
            eth,
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
            let (_eloop, transport) = web3::transports::Http::new(p.eth.eth_url.as_str()).unwrap();
            let web3 = web3::Web3::new(transport);
            let acc = web3.personal().new_account("123").wait().unwrap();
            p.eth.account = acc;

            if with_pass {
                p.eth.credentials = Credentials::Password(String::from("123"));
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
            p.eth.eth_url = String::from("http://117.2.6.7:4476");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_out_of_gas() -> Result<(), Box<Error>> {
        let publisher = generate_with("fa0de43c68bea2167181cd8a83f990d02a049336", |p| {
            p.eth.gas = 1;
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

    #[test]
    fn publish_pinned_to_unknown_nodes() -> () {
        let mut publisher =
            generate_publisher("64b8f12d14925394ae0119466dff6ff2b021a3e9", Credentials::No);
        publisher.pin_to_nodes = vec![
            "0xbcb36bd30c5d9fa7c4e6c21d07be39e1d617ea0547f0c2eeb9f66619c2500000",
            "0xabdec4300c5d9fa7c4e6c21d07be39e1d617ea0547f0c2eeb9f66619c25aaaaa",
            "0xada710010c5d9fa7c4e6c21d07be39e1d617ea0547f0c2eeb9f66619c25bbbbb",
        ]
        .into_iter()
        .map(|v| v.into())
        .collect();

        let result = publisher.publish(false);
        assert!(result.is_err());

        if let Result::Err(e) = result {
            assert!(e.to_string().contains("Can pin only to registered nodes"))
        }
    }

    // TODO: add tests on successful pinning
}
