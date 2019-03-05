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
use clap::{App, AppSettings, SubCommand};
use web3::transports::Http;

use crate::command;
use crate::config::SetupConfig;
use crate::contract_func::call_contract;
use crate::contract_func::contract::functions::delete_app;
use crate::contract_func::contract::functions::delete_node;
use crate::contract_func::contract::functions::dequeue_app;
use crate::contract_status::status;
use crate::ethereum_params::EthereumParams;
use failure::{Error, SyncFailure};
use web3::futures::Future;

#[derive(Debug)]
pub struct DeleteAll {
    eth: EthereumParams,
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("delete_all")
        .about("Delete all apps and nodes from contract. For the test net for contract owner only.")
        .args(command::with_ethereum_args(&[]).as_slice())
        .setting(AppSettings::Hidden)
        .setting(AppSettings::ArgRequiredElseHelp)
}

pub fn parse(args: &ArgMatches, config: &SetupConfig) -> Result<DeleteAll, Error> {
    let eth = command::parse_ethereum_args(args, config)?;

    return Ok(DeleteAll { eth });
}

impl DeleteAll {
    pub fn new(eth: EthereumParams) -> DeleteAll {
        DeleteAll { eth }
    }

    /// Deletes all nodes and apps from contract.
    pub fn delete_all(self) -> Result<(), Error> {
        let (_eloop, transport) = Http::new(self.eth.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = &web3::Web3::new(transport);

        println!("Getting status...");

        let status = status::get_status(web3, self.eth.contract_address)?;

        let apps = status.apps;
        let nodes = status.nodes;

        println!(
            "Status received, going to delete {} nodes and {} apps",
            nodes.len(),
            apps.len()
        );

        let mut nonce = web3
            .eth()
            .transaction_count(self.eth.account, None)
            .wait()
            .map_err(SyncFailure::new)?;

        for app in apps {
            let call_data = if app.cluster.is_some() {
                delete_app::call(app.app_id).0
            } else {
                dequeue_app::call(app.app_id).0
            };
            call_contract(web3, &self.eth, call_data, Some(nonce))?;
            nonce = nonce + 1;
        }

        println!("All nodes have been deleted.");

        for node in nodes {
            let call_data = delete_node::call(node.validator_key).0;
            call_contract(web3, &self.eth, call_data, Some(nonce))?;
            nonce = nonce + 1;
        }

        println!("All apps have been deleted.");

        Ok(())
    }
}
