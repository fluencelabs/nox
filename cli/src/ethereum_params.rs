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
    pub fn generate(args: &EthereumArgs, config: &SetupConfig) -> Result<EthereumParams, Error> {
        let secret_key = config.secret_key.map(|s| Secret::from(s));

        let creds = args.credentials.clone();
        let creds = match creds {
            Credentials::No => credentials::load_credentials(
                config.keystore_path.clone(),
                config.password.clone(),
                secret_key,
            )?,
            other => other,
        };

        let contract_address = args
            .contract_address
            .unwrap_or(config.contract_address.clone());

        let account = args
            .account
            .or_else(|| config.account)
            .ok_or_else(|| err_msg("Specify account address in config or in argument"))?;

        let eth_url = args.eth_url.clone().unwrap_or(config.eth_url.clone());

        Ok(EthereumParams {
            credentials: creds,
            gas: args.gas,
            gas_price: args.gas_price,
            account: account,
            contract_address: contract_address,
            eth_url: eth_url,
            wait_tx_include: args.wait_tx_include,
            wait_eth_sync: args.wait_eth_sync,
        })
    }
}
