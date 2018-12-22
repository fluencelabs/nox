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

use ethcore_transaction::{Action, Transaction};
use ethkey::Secret;
use std::error::Error;
use utils;
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::contract::Options;
use web3::futures::Future;
use web3::types::{Address, Bytes, H256};
use web3::Web3;
use web3::transports::Http;
use credentials::Credentials;


pub struct ContractFunc {
    eth_url: String,
    contract_address: Address,
}

impl ContractFunc {
    pub fn new(contract_address: Address, eth_url: &str) -> Result<ContractFunc, Box<Error>> {
        let eth_url = eth_url.to_owned();
        Ok(ContractFunc {
            eth_url,
            contract_address,
        })
    }

    pub fn call_contract_new<P>(
        &self,
        account: Address,
        credentials: &Credentials,
        func: &str,
        params: P,
        gas: u32,
    ) -> Result<H256, Box<Error>>
        where
            P: Tokenize,
    {
        let (_eloop, transport) = Http::new(&self.eth_url)?;
        let web3 = web3::Web3::new(transport);

        match credentials {
            Credentials::No() => {
                self.call_contract(web3, account, None, utils::options_with_gas(gas), func, params)
            },
            Credentials::Password(pass) => {
                self.call_contract(web3, account, Some(&pass), utils::options_with_gas(gas), func, params)
            },
            Credentials::Secret(secret) => {
                self.call_contract_trusted(web3, account, &secret, func, params)
            },
        }
    }

    fn call_contract_trusted<P>(
        &self,
        web3: Web3<Http>,
        account: Address,
        secret: &Secret,
        func: &str,
        params: P,
    ) -> Result<H256, Box<Error>>
    where
        P: Tokenize,
    {
        let raw_contract = utils::init_raw_contract()?;

        let func = raw_contract.function(func)?;

        let encoded = func.encode_input(&params.into_tokens())?;

        let gas_price = web3.eth().gas_price().wait()?;

        let tx = Transaction {
            nonce: web3.eth().transaction_count(account, None).wait()?,
            value: "0".parse()?,
            action: Action::Call(self.contract_address.clone()),
            data: encoded,
            gas: 300000.into(),
            gas_price: gas_price,
        };

        //    let priv_key: H256 = "cb0799337df06a6c73881bab91304a68199a430ccd4bc378e37e51fd1b118133".parse()?;
        //    let secret = Secret::from(priv_key);

        let tx_signed = tx.sign(secret, None);

        let resp = web3
            .eth()
            .send_raw_transaction(Bytes(rlp::encode(&tx_signed).to_vec()))
            .wait()?;

        Ok(resp)
    }

    /// Calls contract method and returns hash of the transaction
    fn call_contract<P>(
        &self,
        web3: Web3<Http>,
        account: Address,
        password: Option<&str>,
        options: Options,
        func: &str,
        params: P,
    ) -> Result<H256, Box<Error>>
    where
        P: Tokenize,
    {
        if let Some(p) = password {
                web3.personal()
                .unlock_account(account, p, None)
                .wait()?;
        }

        let contract = utils::init_contract(&web3, self.contract_address)?;

        let result_code_publish = contract.call(func, params, account, options);
        let res = result_code_publish.wait().map_err(|er| {
            println!("{:?}", er);
            er
        });
        Ok(res?)
    }

    /// Calls contract method and returns some result
    pub fn query_contract<P, R>(&self, func: &str, params: P) -> Result<R, Box<Error>>
    where
        P: Tokenize,
        R: Detokenize,
    {
        let (_eloop, transport) = web3::transports::Http::new(&self.eth_url)?;
        let web3 = web3::Web3::new(transport);

        let contract = utils::init_contract(&web3, self.contract_address)?;

        let result_code_publish = contract.query(func, params, None, utils::options(), None);
        let res = result_code_publish.wait()?;
        Ok(res)
    }
}
