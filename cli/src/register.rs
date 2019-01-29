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

use failure::err_msg;
use failure::Error;

use clap::{value_t, App, Arg, ArgMatches, SubCommand};
use derive_getters::Getters;
use hex;
use web3::types::H256;

use crate::command::*;
use crate::contract_func::contract::events::app_deployed::parse_log as parse_deployed;
use crate::contract_func::contract::functions::add_node;
use crate::contract_func::{call_contract, get_transaction_logs, wait_sync, wait_tx_included};
use crate::types::{NodeAddress, IP_LEN, TENDERMINT_NODE_ID_LEN};
use crate::utils;
use web3::types::H160;

const NODE_IP: &str = "node_ip";
const START_PORT: &str = "start_port";
const LAST_PORT: &str = "last_port";
const PRIVATE: &str = "private";

#[derive(Debug, Getters)]
pub struct Register {
    node_ip: IpAddr,
    tendermint_key: H256,
    tendermint_node_id: H160,
    start_port: u16,
    last_port: u16,
    private: bool,
    eth: EthereumArgs,
}

pub enum Registered {
    TransactionSent(H256),
    Deployed {
        app_ids: Vec<u64>,
        ports: Vec<u16>,
        tx: H256,
    },
    Enqueued(H256),
}

impl Register {
    /// Creates `Register` structure
    pub fn new(
        node_address: IpAddr,
        tendermint_key: H256,
        tendermint_node_id: H160,
        start_port: u16,
        last_port: u16,
        private: bool,
        eth: EthereumArgs,
    ) -> Result<Register, Error> {
        if last_port < start_port {
            return Err(err_msg("last_port should be bigger than start_port"));
        }

        Ok(Register {
            node_ip: node_address,
            tendermint_key,
            tendermint_node_id,
            start_port,
            last_port,
            private,
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
        let publish_to_contract_fn = || -> Result<H256, Error> {
            let hash_addr: NodeAddress = self.serialize_node_address()?;

            let (call_data, _) = add_node::call(
                self.tendermint_key,
                hash_addr,
                u64::from(self.start_port),
                u64::from(self.last_port),
                self.private,
            );

            call_contract(&self.eth, call_data)
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
            let sync_inc = self.eth.wait_eth_sync as u32;
            let steps = 1 + (self.eth.wait_tx_include as u32) + sync_inc;
            let step = |s| format!("{}/{}", s + sync_inc, steps);

            if self.eth.wait_eth_sync {
                utils::with_progress(
                    "Waiting while Ethereum node is syncing...",
                    step(0).as_str(),
                    "Ethereum node synced.",
                    || wait_sync(self.eth.eth_url.clone()),
                )?;
            };

            let tx = utils::with_progress(
                "Registering the node in the smart contract...",
                step(1).as_str(),
                "Transaction with node registration was sent.",
                publish_to_contract_fn,
            )?;

            if self.eth.wait_tx_include {
                utils::print_tx_hash(tx);
                utils::with_progress(
                    "Waiting for the transaction to be included in a block...",
                    step(2).as_str(),
                    "Transaction was included.",
                    || {
                        wait_tx_included(self.eth.eth_url.clone(), &tx)?;
                        wait_event_fn(&tx)
                    },
                )
            } else {
                Ok(Registered::TransactionSent(tx))
            }
        } else {
            if self.eth.wait_eth_sync {
                wait_sync(self.eth.eth_url.clone())?;
            }

            let tx = publish_to_contract_fn()?;

            if self.eth.wait_tx_include {
                wait_event_fn(&tx)
            } else {
                Ok(Registered::TransactionSent(tx))
            }
        }
    }
}

pub fn parse(args: &ArgMatches) -> Result<Register, Error> {
    let node_address: IpAddr = value_t!(args, NODE_IP, IpAddr)?;

    let tendermint_key: H256 = parse_tendermint_key(args)?;

    let tendermint_node_id: H160 = parse_tendermint_node_id(args)?;

    let start_port = value_t!(args, START_PORT, u16)?;
    let last_port = value_t!(args, LAST_PORT, u16)?;

    let private: bool = args.is_present(PRIVATE);

    let eth = parse_ethereum_args(args)?;

    Register::new(
        node_address,
        tendermint_key,
        tendermint_node_id,
        start_port,
        last_port,
        private,
        eth,
    )
}

/// Parses arguments from console and initialize parameters for Publisher
pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(NODE_IP)
            .long(NODE_IP)
            .short("i")
            .required(true)
            .takes_value(true)
            .help("Node's IP address"),
        tendermint_key(),
        tendermint_node_id(),
        Arg::with_name(START_PORT)
            .alias(START_PORT)
            .long(START_PORT)
            .default_value("20096")
            .takes_value(true)
            .help("Minimum port in the port range"),
        Arg::with_name(LAST_PORT)
            .alias(LAST_PORT)
            .default_value("20196")
            .long(LAST_PORT)
            .takes_value(true)
            .help("Maximum port in the port range"),
        base64_tendermint_key(),
        Arg::with_name(PRIVATE)
            .long(PRIVATE)
            .short("p")
            .takes_value(false)
            .help("Marks node as private, used for pinning apps to nodes"),
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
