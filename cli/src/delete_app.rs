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

use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use web3::types::H256;

use std::error::Error;

use crate::command;
use crate::contract_func::contract::functions::delete_app;
use crate::contract_func::contract::functions::dequeue_app;
use crate::contract_func::ContractCaller;
use crate::utils;

const APP_ID: &str = "app_id";
const DEPLOYED: &str = "deployed";

#[derive(Debug)]
pub struct DeleteApp {
    app_id: H256,
    deployed: bool,
    eth: command::EthereumArgs,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    let my_args = &[
        Arg::with_name(DEPLOYED)
            .long(DEPLOYED)
            .short("D")
            .required(false)
            .takes_value(false)
            .help("if not specified, enqueued app will be dequeued, otherwise deployed app will be removed"),
        Arg::with_name(APP_ID)
            .required(true)
            .takes_value(true)
            .help("app to be removed")
    ];

    SubCommand::with_name("delete_app")
        .about("Delete app from smart-contract")
        .args(command::with_ethereum_args(my_args).as_slice())
}

pub fn parse(args: &ArgMatches) -> Result<DeleteApp, Box<Error>> {
    let app_id: H256 = utils::parse_hex_opt(args, APP_ID)?.parse()?;
    let deployed = args.is_present(DEPLOYED);

    let eth = command::parse_ethereum_args(args)?;

    return Ok(DeleteApp {
        app_id,
        deployed,
        eth,
    });
}

impl DeleteApp {
    pub fn new(app_id: H256, deployed: bool, eth: command::EthereumArgs) -> DeleteApp {
        DeleteApp {
            app_id,
            deployed,
            eth,
        }
    }

    pub fn delete_app(self, show_progress: bool) -> Result<H256, Error> {
        let delete_app_fn = || -> Result<H256, Error> {
            let call_data = match self.deployed {
                true => delete_app::call(self.app_id).0,
                false => dequeue_app::call(self.app_id).0,
            };

            let contract =
                ContractCaller::new(self.eth.contract_address, &self.eth.eth_url.as_str())?;

            contract.call_contract(
                self.eth.account,
                &self.eth.credentials,
                call_data,
                self.eth.gas,
            )
        };

        if show_progress {
            utils::with_progress(
                "Deleting app from smart contract...",
                "1/1",
                "App deleted.",
                delete_app_fn,
            )
        } else {
            delete_app_fn()
        }
    }
}
