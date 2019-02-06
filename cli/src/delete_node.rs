/*
 * Copyright 2019 Fluence Labs Limited
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

use clap::ArgMatches;
use clap::{App, SubCommand};
use web3::types::H256;

use crate::contract_func::contract::events::node_deleted;

use crate::command::*;
use crate::contract_func::call_contract;
use crate::utils;
use failure::{err_msg, Error, SyncFailure};
use web3::transports::Http;

use crate::contract_func::contract::functions::delete_node;
use crate::contract_func::get_transaction_logs;
use crate::contract_func::wait_sync;
use crate::contract_func::wait_tx_included;

pub struct DeleteNode {
    tendermint_key: H256,
    eth: EthereumArgs,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        tendermint_key().display_order(0),
        base64_tendermint_key().display_order(1),
    ];
    SubCommand::with_name("delete_node")
        .about("Delete node from smart-contract")
        .args(with_ethereum_args(args).as_slice())
}

pub fn parse(args: &ArgMatches) -> Result<DeleteNode, Error> {
    let tendermint_key = parse_tendermint_key(args)?;
    let eth = parse_ethereum_args(args)?;

    Ok(DeleteNode {
        tendermint_key,
        eth,
    })
}

impl DeleteNode {
    pub fn new(tendermint_key: H256, eth: EthereumArgs) -> DeleteNode {
        DeleteNode {
            tendermint_key,
            eth,
        }
    }

    pub fn delete_node(self, show_progress: bool) -> Result<H256, Error> {
        let (_eloop, transport) = Http::new(self.eth.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = &web3::Web3::new(transport);

        let delete_node_fn = || -> Result<H256, Error> {
            let (call_data, _) = delete_node::call(self.tendermint_key);

            call_contract(web3, &self.eth, call_data)
        };

        let wait_event_fn = |tx: &H256| -> Result<(), Error> {
            let logs =
                get_transaction_logs(self.eth.eth_url.as_str(), tx, node_deleted::parse_log)?;
            logs.first().ok_or(err_msg(format!(
                "No NodeDeleted event is found in transaction logs. tx: {:#x}",
                tx
            )))?;

            Ok(())
        };

        if show_progress {
            let sync_inc = self.eth.wait_eth_sync as u32;
            let steps = 1 + (self.eth.wait_tx_include as u32) + sync_inc;
            let step = |s| format!("{}/{}", s + sync_inc, steps);

            if self.eth.wait_eth_sync {
                utils::with_progress(
                    "Waiting while Ethereum node is syncing...",
                    step(0).as_str(),
                    "Ethereum node synced.",
                    || wait_sync(web3),
                )?;
            }

            let tx = utils::with_progress(
                "Deleting node from smart contract...",
                step(1).as_str(),
                "Node deleted.",
                delete_node_fn,
            )?;

            if self.eth.wait_tx_include {
                utils::print_tx_hash(tx);
                utils::with_progress(
                    "Waiting for a transaction to be included in a block...",
                    step(2).as_str(),
                    "Transaction included. App deleted.",
                    || {
                        wait_tx_included(&tx, web3)?;
                        wait_event_fn(&tx)?;
                        Ok(tx)
                    },
                )
            } else {
                Ok(tx)
            }
        } else {
            if self.eth.wait_eth_sync {
                wait_sync(web3)?;
            }
            let tx = delete_node_fn()?;

            if self.eth.wait_tx_include {
                wait_tx_included(&tx, web3)?;
                wait_event_fn(&tx)?;
            }

            Ok(tx)
        }
    }
}
