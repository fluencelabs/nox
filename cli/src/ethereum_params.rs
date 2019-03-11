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

use crate::command::EthereumArgs;
use crate::config::SetupConfig;
use crate::credentials;
use crate::credentials::Credentials;
use ethkey::Secret;
use failure::err_msg;
use failure::Error;
use web3::types::Address;

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
    pub fn generate(args: EthereumArgs, config: SetupConfig) -> Result<EthereumParams, Error> {
        let secret_key = config.secret_key.map(|s| Secret::from(s));

        let credentials = args.credentials;
        let credentials = match credentials {
            Credentials::No => {
                credentials::load_credentials(config.keystore_path, config.password, secret_key)?
            }
            other => other,
        };

        let contract_address = args.contract_address.unwrap_or(config.contract_address);

        let account = args
            .account
            .or(config.account)
            .ok_or_else(|| err_msg("Account address is not defined. Use "))?;

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
