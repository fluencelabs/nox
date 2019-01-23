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

use ethabi_contract::use_contract;
use ethcore_transaction::{Action, Transaction};
use ethkey::Secret;
use web3::contract::Options;
use web3::futures::Future;
use web3::transports::Http;
use web3::types::CallRequest;
use web3::types::TransactionRequest;
use web3::types::{Address, Bytes, H256};
use web3::Web3;

use failure::Error;
use failure::SyncFailure;

use crate::credentials::Credentials;
use crate::utils;

use_contract!(contract, "../bootstrap/contracts/compiled/Network.abi");

/// Interacts with contract
pub struct ContractCaller {
    eth_url: String,
    contract_address: Address,
}

impl ContractCaller {
    pub fn new(contract_address: Address, eth_url: &str) -> Result<ContractCaller, Error> {
        let eth_url = eth_url.to_owned();
        Ok(ContractCaller {
            eth_url,
            contract_address,
        })
    }

    /// Calls contract method and returns hash of the transaction
    pub fn call_contract(
        &self,
        account: Address,
        credentials: &Credentials,
        call_data: ethabi::Bytes,
        gas: u32,
    ) -> Result<H256, Error> {
        let (_eloop, transport) = Http::new(&self.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = web3::Web3::new(transport);

        match credentials {
            Credentials::No => self.call_contract_trusted_node(
                web3,
                account,
                None,
                utils::options_with_gas(gas),
                call_data,
            ),
            Credentials::Password(pass) => self.call_contract_trusted_node(
                web3,
                account,
                Some(pass.as_str()),
                utils::options_with_gas(gas),
                call_data,
            ),
            Credentials::Secret(secret) => {
                self.call_contract_local_sign(web3, account, &secret, call_data, gas)
            }
        }
    }

    /// Signs transaction with a secret key and sends a raw transaction to Ethereum node
    fn call_contract_local_sign(
        &self,
        web3: Web3<Http>,
        account: Address,
        secret: &Secret,
        call_data: ethabi::Bytes,
        gas: u32,
    ) -> Result<H256, Error> {
        let gas_price = web3.eth().gas_price().wait().map_err(SyncFailure::new)?;
        let nonce = web3.eth().transaction_count(account, None).wait().map_err(SyncFailure::new)?;

        let tx = Transaction {
            nonce,
            value: "0".parse()?,
            action: Action::Call(self.contract_address.clone()),
            data: call_data,
            gas: gas.into(),
            gas_price,
        };

        let tx_signed = tx.sign(secret, None);

        let resp = web3
            .eth()
            .send_raw_transaction(Bytes(rlp::encode(&tx_signed).to_vec()))
            .wait()
            .map_err(SyncFailure::new)?;

        Ok(resp)
    }

    /// Sends a transaction to a trusted node with an unlocked account (or, firstly, unlocks account with password)
    fn call_contract_trusted_node(
        &self,
        web3: Web3<Http>,
        account: Address,
        password: Option<&str>,
        options: Options,
        call_data: ethabi::Bytes,
    ) -> Result<H256, Error> {
        let tx_request = TransactionRequest {
            from: account,
            to: Some(self.contract_address.clone()),
            data: Some(Bytes(call_data)),
            gas: options.gas,
            gas_price: options.gas_price,
            value: options.value,
            nonce: options.nonce,
            condition: options.condition,
        };

        let result = match password {
            Some(p) => web3.personal().send_transaction(tx_request, p),
            None => web3.eth().send_transaction(tx_request),
        };

        Ok(result.wait().map_err(SyncFailure::new)?)
    }

    /// Calls contract method and returns some result
    pub fn query_contract<R>(
        &self,
        call_data: ethabi::Bytes,
        decoder: Box<R>,
    ) -> Result<R::Output, Error>
    where
        R: ethabi::FunctionOutputDecoder,
    {
        let (_eloop, transport) =
            web3::transports::Http::new(&self.eth_url.as_str()).map_err(SyncFailure::new)?;
        let web3 = web3::Web3::new(transport);
        let call_request = CallRequest {
            to: self.contract_address.clone(),
            data: Some(Bytes(call_data)),
            gas: None,
            gas_price: None,
            value: None,
            from: None,
        };
        let result = web3
            .eth()
            .call(call_request, None)
            .wait()
            .map_err(SyncFailure::new)?;
        let result: <R as ethabi::FunctionOutputDecoder>::Output = decoder
            .decode(result.0.as_slice())
            .map_err(SyncFailure::new)?;

        Ok(result)
    }
}
