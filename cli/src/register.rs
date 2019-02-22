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

use std::net::IpAddr;

use failure::{err_msg, Error, SyncFailure};

use clap::{value_t, App, Arg, ArgMatches, SubCommand};
use derive_getters::Getters;
use hex;
use web3::types::H256;

use crate::command::*;
use crate::contract_func::contract::events::app_deployed::parse_log as parse_deployed;
use crate::contract_func::contract::functions::add_node;
use crate::contract_func::{call_contract, get_transaction_logs, wait_sync, wait_tx_included};
use crate::contract_status::{find_by_tendermint_key, status};
use crate::step_counter::StepCounter;
use crate::types::{NodeAddress, IP_LEN, TENDERMINT_NODE_ID_LEN};
use crate::utils;
use web3::transports::Http;
use web3::types::H160;

const API_PORT: &str = "api_port";
const CAPACITY: &str = "capacity";
const PRIVATE: &str = "private";
const NO_STATUS_CHECK: &str = "no_status_check";

#[derive(Debug, Getters)]
pub struct Register {
    node_ip: IpAddr,
    tendermint_key: H256,
    tendermint_node_id: H160,
    api_port: u16,
    capacity: u16,
    private: bool,
    no_status_check: bool,
    eth: EthereumArgs,
}

#[derive(PartialEq, Debug)]
pub enum Registered {
    TransactionSent(H256),
    Deployed {
        app_ids: Vec<u64>,
        ports: Vec<u16>,
        tx: H256,
    },
    Enqueued(H256),
    AlreadyRegistered,
}

impl Register {
    /// Creates `Register` structure
    pub fn new(
        node_address: IpAddr,
        tendermint_key: H256,
        tendermint_node_id: H160,
        api_port: u16,
        capacity: u16,
        private: bool,
        no_status_check: bool,
        eth: EthereumArgs,
    ) -> Result<Register, Error> {
        Ok(Register {
            node_ip: node_address,
            tendermint_key,
            tendermint_node_id,
            api_port,
            capacity,
            private,
            no_status_check,
            eth,
        })
    }

    /// Serializes a node IP address and a tendermint key into the hash of node's key address
    fn serialize_node_address(&self) -> Result<NodeAddress, Error> {
        // serialize tendermint key
        let key_str = format!("{:?}", self.tendermint_node_id);
        let key_str = key_str.as_str().trim_start_matches("0x");

        let key_bytes = hex::decode(key_str.to_owned())?;
        let mut key_bytes = key_bytes.as_slice()[0..TENDERMINT_NODE_ID_LEN].to_vec();

        // serialize IP address
        let ip_str = self.node_ip.to_string();
        let split = ip_str.split('.');
        let mut addr_bytes: [u8; IP_LEN] = [0; IP_LEN];
        for (i, part) in split.enumerate() {
            addr_bytes[i] = part.parse()?;
        }
        let mut addr_vec = addr_bytes.to_vec();

        // concatenate tendermint key and IP address
        key_bytes.append(&mut addr_vec);

        let serialized = hex::encode(key_bytes);

        let hash_addr: NodeAddress = serialized.parse()?;

        Ok(hash_addr)
    }

    /// Registers a node in Fluence smart contract
    pub fn register(&self, show_progress: bool) -> Result<Registered, Error> {
        let (_eloop, transport) = Http::new(self.eth.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = &web3::Web3::new(transport);

        let publish_to_contract_fn = || -> Result<H256, Error> {
            let hash_addr: NodeAddress = self.serialize_node_address()?;

            let (call_data, _) = add_node::call(
                self.tendermint_key,
                hash_addr,
                u64::from(self.api_port),
                u64::from(self.capacity),
                self.private,
            );

            call_contract(web3, &self.eth, call_data, None)
        };

        let check_node_registered_fn = || -> Result<bool, Error> {
            let contract_status = status::get_status(web3, self.eth.contract_address)?;
            Ok(find_by_tendermint_key(&contract_status, self.tendermint_key).is_some())
        };

        let wait_event_fn = |tx: &H256| -> Result<Registered, Error> {
            let logs = get_transaction_logs(self.eth.eth_url.as_str(), tx, parse_deployed)?;

            let status = if logs.is_empty() {
                Registered::Enqueued(*tx)
            } else {
                let (app_ids, ports) = logs
                    .iter()
                    .filter_map(|e| {
                        let idx = e.node_i_ds.iter().position(|id| *id == self.tendermint_key);
                        idx.and_then(|i| e.ports.get(i)).map(|port| {
                            let port = Into::<u64>::into(*port) as u16;
                            let app_id = Into::<u64>::into(e.app_id);
                            (app_id, port)
                        })
                    })
                    .unzip();
                Registered::Deployed {
                    app_ids,
                    ports,
                    tx: *tx,
                }
            };

            Ok(status)
        };

        // sending transaction with the hash of file with code to ethereum
        if show_progress {
            let mut step_counter = StepCounter::new(1);
            if !self.no_status_check {
                step_counter.register()
            };
            if self.eth.wait_eth_sync {
                step_counter.register()
            };
            if self.eth.wait_tx_include {
                step_counter.register()
            };

            if self.eth.wait_eth_sync {
                utils::with_progress(
                    "Waiting while Ethereum node is syncing...",
                    step_counter.format_next_step().as_str(),
                    "Ethereum node synced.",
                    || wait_sync(web3),
                )?;
            };

            let is_registered = if !self.no_status_check {
                utils::with_progress(
                    "Check if node in smart contract is already registered...",
                    step_counter.format_next_step().as_str(),
                    "Smart contract checked.",
                    || check_node_registered_fn(),
                )?
            } else {
                false
            };

            if is_registered {
                Ok(Registered::AlreadyRegistered)
            } else {
                let tx = utils::with_progress(
                    "Registering the node in the smart contract...",
                    step_counter.format_next_step().as_str(),
                    "Transaction with node registration was sent.",
                    publish_to_contract_fn,
                )?;

                if self.eth.wait_tx_include {
                    utils::print_tx_hash(tx);
                    utils::with_progress(
                        "Waiting for the transaction to be included in a block...",
                        step_counter.format_next_step().as_str(),
                        "Transaction was included.",
                        || {
                            wait_tx_included(&tx, web3)?;
                            wait_event_fn(&tx)
                        },
                    )
                } else {
                    Ok(Registered::TransactionSent(tx))
                }
            }
        } else {
            if self.eth.wait_eth_sync {
                wait_sync(web3)?;
            }

            let is_registered = if !self.no_status_check {
                check_node_registered_fn()?
            } else {
                false
            };

            if is_registered {
                Ok(Registered::AlreadyRegistered)
            } else {
                let tx = publish_to_contract_fn()?;

                if self.eth.wait_tx_include {
                    wait_event_fn(&tx)
                } else {
                    Ok(Registered::TransactionSent(tx))
                }
            }
        }
    }
}

pub fn parse(args: &ArgMatches) -> Result<Register, Error> {
    let tendermint_key: H256 = parse_tendermint_key(args)?;

    let tendermint_node_id: H160 = parse_tendermint_node_id(args)?;

    let api_port = value_t!(args, API_PORT, u16)?;
    let capacity = value_t!(args, CAPACITY, u16)?;

    let private: bool = args.is_present(PRIVATE);
    let no_status_check: bool = args.is_present(NO_STATUS_CHECK);

    let eth = parse_ethereum_args(args)?;

    let node_address = parse_node_ip(&args)?;

    Register::new(
        node_address,
        tendermint_key,
        tendermint_node_id,
        api_port,
        capacity,
        private,
        no_status_check,
        eth,
    )
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        node_ip().display_order(1),
        tendermint_key().display_order(2),
        base64_tendermint_key().display_order(3),
        tendermint_node_id().display_order(4),
        Arg::with_name(API_PORT)
            .alias(API_PORT)
            .long(API_PORT)
            .default_value("20096")
            .takes_value(true)
            .help("Node API port")
            .display_order(5),
        Arg::with_name(CAPACITY)
            .alias(CAPACITY)
            .default_value("20196")
            .long(CAPACITY)
            .takes_value(true)
            .help("Maximum number of apps to be run on the node")
            .display_order(5),
        Arg::with_name(PRIVATE)
            .long(PRIVATE)
            .short("p")
            .takes_value(false)
            .help("Marks node as private, used for pinning apps to nodes")
            .display_order(6),
        Arg::with_name(NO_STATUS_CHECK)
            .long(NO_STATUS_CHECK)
            .short("N")
            .takes_value(false)
            .help("Disable checking if a node is already registered. Registration can be faster but will spend some gas if the node is registered.")
            .display_order(7),
    ];

    SubCommand::with_name("register")
        .about("Register a node in the smart contract")
        .args(with_ethereum_args(args).as_slice())
}

#[cfg(test)]
pub mod tests {
    use failure::Error;

    use ethkey::Secret;
    use rand::prelude::*;
    use web3::types::*;

    use crate::command::EthereumArgs;
    use crate::credentials::Credentials;

    use super::Register;

    pub fn generate_register(credentials: Credentials) -> Register {
        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();

        let tendermint_key: H256 = H256::from(rnd_num);
        let tendermint_node_id: H160 = H160::from(rnd_num);
        let account: Address = "4180fc65d613ba7e1a385181a219f1dbfe7bf11d".parse().unwrap();

        let eth = EthereumArgs::with_acc_creds(account, credentials);

        Register::new(
            "127.0.0.1".parse().unwrap(),
            tendermint_key,
            tendermint_node_id,
            25006,
            25100,
            false,
            false,
            eth,
        )
        .unwrap()
    }

    pub fn generate_with<F>(func: F, credentials: Credentials) -> Register
    where
        F: FnOnce(&mut Register),
    {
        let mut register = generate_register(credentials);
        func(&mut register);
        register
    }

    pub fn generate_with_account(account: Address, credentials: Credentials) -> Register {
        generate_with(|r| r.eth.account = account, credentials)
    }

    #[test]
    fn register_success() -> Result<(), Error> {
        let register = generate_with_account(
            "fa0de43c68bea2167181cd8a83f990d02a049336".parse()?,
            Credentials::No,
        );

        register.register(false)?;

        Ok(())
    }

    #[test]
    fn register_out_of_gas() -> Result<(), Error> {
        let register = generate_with(|r| r.eth.gas = 1, Credentials::No);

        let result = register.register(false);

        assert_eq!(result.is_err(), true);

        Ok(())
    }

    #[test]
    fn register_success_with_secret() -> Result<(), Error> {
        let secret_arr: H256 =
            "a349fe22d5c6f8ad3a1ad91ddb65e8946435b52254ce8c330f7ed796e83bfd92".parse()?;
        let secret = Secret::from(secret_arr);
        let register = generate_with_account(
            "dce48d51717ad5eb87fb56ff55ec609cf37b9aad".parse()?,
            Credentials::Secret(secret),
        );

        register.register(false)?;

        Ok(())
    }
}
