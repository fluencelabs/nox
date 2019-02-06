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

use std::fs::File;
use std::io::prelude::*;

use failure::{err_msg, Error, ResultExt, SyncFailure};

use clap::ArgMatches;
use clap::{value_t, App, Arg, SubCommand};
use derive_getters::Getters;
use ethabi::RawLog;
use reqwest::Client;
use web3::transports::Http;
use web3::types::H256;

use crate::command::{parse_ethereum_args, with_ethereum_args, EthereumArgs};
use crate::contract_func::contract::events::app_deployed::parse_log as parse_deployed;
use crate::contract_func::contract::events::app_enqueued::parse_log as parse_enqueued;
use crate::contract_func::contract::functions::add_app;
use crate::contract_func::{call_contract, get_transaction_logs_raw, wait_tx_included};
use crate::utils;

const CODE_PATH: &str = "code_path";
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

#[derive(Debug)]
pub enum Published {
    TransactionSent(H256),
    Deployed { app_id: u64, tx: H256 },
    Enqueued { app_id: u64, tx: H256 },
}

impl Published {
    pub fn deployed(app_id: u64, tx: H256) -> Published {
        Published::Deployed { app_id, tx }
    }
    pub fn enqueued(app_id: u64, tx: H256) -> Published {
        Published::Enqueued { app_id, tx }
    }
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
    pub fn publish(&self, show_progress: bool) -> Result<Published, Error> {
        let (_eloop, transport) = Http::new(self.eth.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = &web3::Web3::new(transport);

        let upload_to_swarm_fn = || -> Result<H256, Error> {
            let hash = upload_code_to_swarm(&self.swarm_url.as_str(), &self.bytes.as_slice())?;
            let hash = hash.parse()?;
            Ok(hash)
        };

        let publish_to_contract_fn = |hash: H256| -> Result<H256, Error> {
            //todo: add correct receipts
            let receipt: H256 =
                "0000000000000000000000000000000000000000000000000000000000000000".parse()?;

            let (call_data, _) = add_app::call(
                hash,
                receipt,
                u64::from(self.cluster_size),
                self.pin_to_nodes.clone(),
            );

            Ok(call_contract(web3, &self.eth, call_data)
                .context("error calling addApp in smart contract")?)
        };

        let wait_event_fn = |tx: &H256| -> Result<Published, Error> {
            let logs: Vec<Published> =
                get_transaction_logs_raw(self.eth.eth_url.as_str(), &tx, |log| {
                    let raw = || RawLog::from((log.topics.clone(), log.data.0.clone()));

                    let app_id =
                        parse_deployed(raw()).map(|e| Published::deployed(e.app_id.into(), *tx));
                    let app_id = app_id
                        .or(parse_enqueued(raw())
                            .map(|e| Published::enqueued(e.app_id.into(), *tx)));

                    app_id.ok()
                })
                .context("Error parsing transaction logs")?;

            logs.into_iter().nth(0).ok_or(err_msg(format!(
                "No AppDeployed or AppEnqueued event is found in transaction logs. tx: {:#x}",
                tx
            )))
        };

        // sending transaction with the hash of file with code to ethereum
        if show_progress {
            let steps = if self.eth.wait_tx_include { 3 } else { 2 };
            let step = |s| format!("{}/{}", s, steps);

            let hash: H256 = utils::with_progress(
                "Uploading application code to Swarm...",
                step(1).as_str(),
                "Application code uploaded.",
                upload_to_swarm_fn,
            )?;
            utils::print_info_id("swarm hash:", hash);

            let tx = utils::with_progress(
                "Publishing the app to the smart contract...",
                step(2).as_str(),
                "Transaction publishing app was sent.",
                || publish_to_contract_fn(hash),
            )?;

            if self.eth.wait_tx_include {
                utils::print_tx_hash(tx);
                utils::with_progress(
                    "Waiting for a transaction to be included in a block...",
                    step(3).as_str(),
                    "Transaction was included.",
                    || {
                        wait_tx_included(&tx, web3)?;
                        wait_event_fn(&tx)
                    },
                )
            } else {
                Ok(Published::TransactionSent(tx))
            }
        } else {
            let hash = upload_to_swarm_fn()?;
            let tx = publish_to_contract_fn(hash)?;

            if self.eth.wait_tx_include {
                wait_tx_included(&tx, web3)?;
                wait_event_fn(&tx)
            } else {
                Ok(Published::TransactionSent(tx))
            }
        }
    }
}

fn parse_pinned(args: &ArgMatches) -> Result<Vec<H256>, Error> {
    let pin_to_nodes = args.values_of(PINNED).unwrap_or_default();

    let pin_to_nodes = pin_to_nodes.into_iter();

    let pin_to_nodes: Result<Vec<H256>, Error> = pin_to_nodes
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

    Ok(pin_to_nodes.context(format!("unable to parse {}", PINNED))?)
}

/// Creates `Publisher` from arguments
pub fn parse(matches: &ArgMatches) -> Result<Publisher, Error> {
    let path = value_t!(matches, CODE_PATH, String)?; //TODO use is_file from clap_validators
    let mut file = File::open(path).context("can't open WASM file")?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let swarm_url = value_t!(matches, SWARM_URL, String)?;
    let cluster_size = value_t!(matches, CLUSTER_SIZE, u8)?;
    let eth = parse_ethereum_args(matches)?;

    let pin_to_nodes = parse_pinned(matches)?;
    if pin_to_nodes.len() > 0 && pin_to_nodes.len() > (cluster_size as usize) {
        return Err(err_msg(
            "number of pin_to nodes should be less or equal to the desired cluster_size",
        ));
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
    let args = &[
        Arg::with_name(CODE_PATH)
            .long(CODE_PATH)
            .short("c")
            .required(true)
            .takes_value(true)
            .help("Path to compiled `wasm` code")
            .display_order(0),
        Arg::with_name(CLUSTER_SIZE)
            .long(CLUSTER_SIZE)
            .short("s")
            .required(false)
            .takes_value(true)
            .default_value("3")
            .help("Cluster's size that needed to deploy this code")
            .display_order(1),
        Arg::with_name(PINNED)
            .long(PINNED)
            .short("p")
            .required(false)
            .takes_value(true)
            .multiple(true)
            .value_name("key")
            .help("Tendermint public keys of pinned workers for application (space-separated list)")
            .display_order(2),
        Arg::with_name(PIN_BASE64)
            .long(PIN_BASE64)
            .required(false)
            .takes_value(false)
            .help("If specified, tendermint keys for pin_to flag treated as base64")
            .display_order(2),
        Arg::with_name(SWARM_URL)
            .long(SWARM_URL)
            .short("w")
            .required(false)
            .takes_value(true)
            .help("Http address to swarm node")
            .default_value("http://localhost:8500/")
            .display_order(3),
    ];

    SubCommand::with_name("publish")
        .about("Upload code to Swarm and publish app to Ethereum blockchain")
        .args(with_ethereum_args(args).as_slice())
}

/// Uploads bytes of code to the Swarm
fn upload_code_to_swarm(url: &str, bytes: &[u8]) -> Result<String, Error> {
    let mut url = utils::parse_url(url)?;
    url.set_path("/bzz:/");

    let client = Client::new();
    let res = client
        .post(url)
        .body(bytes.to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .and_then(|mut r| r.text())
        .context("error uploading code to swarm")?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use ethkey::Secret;
    use web3;
    use web3::futures::Future;
    use web3::types::H256;

    use failure::Error;

    use crate::command::EthereumArgs;
    use crate::credentials::Credentials;
    use crate::publisher::Publisher;

    const OWNER: &str = "4180FC65D613bA7E1a385181a219F1DBfE7Bf11d";

    fn generate_publisher(account: &str, creds: Credentials) -> Publisher {
        let bytes = vec![1, 2, 3];

        let eth = EthereumArgs::with_acc_creds(account.parse().unwrap(), creds);

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
    fn publish_wrong_password() -> Result<(), Error> {
        let publisher = generate_new_account(false);

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_no_eth() -> Result<(), Error> {
        let publisher = generate_new_account(true);

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_swarm_url() -> Result<(), Error> {
        let publisher = generate_with("02f906f8b3b932fd282109a5b8dc732ba2329888", |p| {
            p.swarm_url = String::from("http://123.5.6.7:8385");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_wrong_eth_url() -> Result<(), Error> {
        let publisher = generate_with("fa0de43c68bea2167181cd8a83f990d02a049336", |p| {
            p.eth.eth_url = String::from("http://117.2.6.7:4476");
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn publish_out_of_gas() -> Result<(), Error> {
        let publisher = generate_with("fa0de43c68bea2167181cd8a83f990d02a049336", |p| {
            p.eth.gas = 1;
        });

        let result = publisher.publish(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    #[ignore]
    fn publish_to_contract_success() -> Result<(), Error> {
        let publisher =
            generate_publisher("64b8f12d14925394ae0119466dff6ff2b021a3e9", Credentials::No);

        publisher.publish(false)?;

        Ok(())
    }

    #[test]
    fn publish_to_contract_with_secret_success() -> Result<(), Error> {
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
            assert!(e
                .find_root_cause()
                .to_string()
                .contains("Can pin only to registered nodes"))
        }
    }
}
