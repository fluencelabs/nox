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

use failure::err_msg;
use failure::Error;
use web3::types::Address;

use crate::command::EthereumArgs;
use crate::command::ACCOUNT;
use crate::config::SetupConfig;
use crate::credentials::Credentials;

// TODO: merge EthereumArgs, SetupConfig and EthereumParams into a single structure
#[derive(Debug, Clone)]
pub struct EthereumParams {
    pub credentials: Credentials,
    pub gas: u32,
    pub gas_price: u64,
    pub account: Address,
    pub contract_address: Address,
    pub eth_url: String,
    pub wait_tx_include: bool,
    pub wait_eth_sync: bool,
}

impl EthereumParams {
    /// Merge command-line arguments with the stored config
    /// specified arguments take precedence over values in config
    pub fn generate(args: EthereumArgs, config: SetupConfig) -> Result<EthereumParams, Error> {
        let credentials = match args.credentials {
            Credentials::No => config.credentials,
            from_args => from_args,
        };

        let contract_address = args.contract_address.unwrap_or(config.contract_address);

        // Account source precedence:
        // 1. --account argument
        // 2. From credentials passed in arguments (see EthereumArgs::parse_ethereum_args)
        // 3. From credentials in config
        let account = args
            .account
            .or(credentials.to_address())
            .ok_or(err_msg(format!(
                "Account address is not defined. Specify it in `setup` command or with `--{}`.",
                ACCOUNT
            )))?;

        let eth_url = args.eth_url.clone().unwrap_or(config.eth_url);

        Ok(EthereumParams {
            credentials,
            gas: args.gas,
            gas_price: args.gas_price,
            account,
            contract_address,
            eth_url,
            wait_tx_include: args.wait_tx_include,
            wait_eth_sync: args.wait_eth_sync,
        })
    }
}
