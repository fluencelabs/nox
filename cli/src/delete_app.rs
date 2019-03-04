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

use clap::value_t;
use clap::ArgMatches;
use clap::{App, AppSettings, Arg, SubCommand};
use web3::transports::Http;
use web3::types::H256;

use crate::command;
use crate::config::SetupConfig;
use crate::contract_func::call_contract;
use crate::contract_func::contract::events::app_deleted;
use crate::contract_func::contract::events::app_dequeued;
use crate::contract_func::contract::functions::delete_app;
use crate::contract_func::contract::functions::dequeue_app;
use crate::contract_func::wait_sync;
use crate::contract_func::{get_transaction_logs, wait_tx_included};
use crate::step_counter::StepCounter;
use crate::utils;
use ethabi::RawLog;
use failure::{err_msg, Error, SyncFailure};

const APP_ID: &str = "app_id";
const DEPLOYED: &str = "deployed";

#[derive(Debug)]
pub struct DeleteApp {
    app_id: u64,
    deployed: bool,
    eth: command::EthereumParams,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let args = &[
        Arg::with_name(APP_ID)
            .long(APP_ID)
            .short("A")
            .required(true)
            .takes_value(true)
            .help("App to be removed")
            .display_order(0),
        Arg::with_name(DEPLOYED)
            .long(DEPLOYED)
            .short("D")
            .required(false)
            .takes_value(false)
            .help("If not specified, enqueued app will be dequeued, otherwise deployed app will be removed")
            .display_order(1),
    ];

    SubCommand::with_name("delete_app")
        .about("Delete app from smart-contract")
        .args(command::with_ethereum_args(args).as_slice())
        .setting(AppSettings::ArgRequiredElseHelp)
}

pub fn parse(args: &ArgMatches, config: SetupConfig) -> Result<DeleteApp, Error> {
    let app_id: u64 = value_t!(args, APP_ID, u64)?;
    let deployed = args.is_present(DEPLOYED);

    let eth = command::parse_ethereum_args(args, config)?;

    return Ok(DeleteApp {
        app_id,
        deployed,
        eth,
    });
}

impl DeleteApp {
    pub fn new(app_id: u64, deployed: bool, eth: command::EthereumParams) -> DeleteApp {
        DeleteApp {
            app_id,
            deployed,
            eth,
        }
    }

    fn check_event<T, F>(&self, tx: &H256, f: F, event_name: &str) -> Result<(), Error>
    where
        F: Fn(RawLog) -> ethabi::Result<T>,
    {
        let logs = get_transaction_logs(self.eth.eth_url.as_str(), tx, f)?;
        logs.first()
            .ok_or(err_msg(format!(
                "No {} event is found in transaction logs. tx: {:#x}",
                event_name, tx
            )))
            .map(|_| ())
    }

    pub fn delete_app(self, show_progress: bool) -> Result<H256, Error> {
        let (_eloop, transport) = Http::new(self.eth.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = &web3::Web3::new(transport);

        let delete_app_fn = || -> Result<H256, Error> {
            let call_data = if self.deployed {
                delete_app::call(self.app_id).0
            } else {
                dequeue_app::call(self.app_id).0
            };

            call_contract(web3, &self.eth, call_data, None)
        };

        let check_event_fn = |tx: &H256| -> Result<(), Error> {
            if self.deployed {
                self.check_event(tx, app_deleted::parse_log, "AppDeleted")
            } else {
                self.check_event(tx, app_dequeued::parse_log, "AppDequeued")
            }
        };

        if show_progress {
            let mut step_counter = StepCounter::new(1);
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
            }

            let tx = utils::with_progress(
                "Deleting app from smart contract...",
                step_counter.format_next_step().as_str(),
                "App deletion transaction was sent.",
                delete_app_fn,
            )?;

            if self.eth.wait_tx_include {
                utils::print_tx_hash(tx);
                utils::with_progress(
                    "Waiting for a transaction to be included in a block...",
                    step_counter.format_next_step().as_str(),
                    "Transaction included. App deleted.",
                    || {
                        wait_tx_included(&tx, web3)?;
                        check_event_fn(&tx)?;
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
            let tx = delete_app_fn()?;

            if self.eth.wait_tx_include {
                check_event_fn(&tx)?;
            }

            Ok(tx)
        }
    }
}
