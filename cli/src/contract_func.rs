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

use crate::command::EthereumArgs;
use ethabi_contract::use_contract;
use ethcore_transaction::{Action, Transaction};
use ethkey::Secret;
use web3::futures::Future;
use web3::transports::Http;
use web3::types::CallRequest;
use web3::types::TransactionRequest;
use web3::types::{Address, Bytes, Transaction as Web3Transaction, H256};
use web3::Web3;

use failure::err_msg;
use failure::Error;
use failure::Fail;
use failure::ResultExt;
use failure::SyncFailure;

use crate::credentials::Credentials;
use crate::utils;
use ethabi::RawLog;
use std::time::Duration;
use web3::types::Log;
use web3::types::TransactionId;

use_contract!(contract, "../bootstrap/contracts/compiled/Network.abi");

/// Calls contract method and returns hash of the transaction
pub fn call_contract(eth: &EthereumArgs, call_data: ethabi::Bytes) -> Result<H256, Error> {
    let (_eloop, transport) = Http::new(&eth.eth_url.as_str()).map_err(SyncFailure::new)?;
    let web3 = web3::Web3::new(transport);

    match &eth.credentials {
        Credentials::No => call_contract_trusted_node(web3, None, call_data, &eth),
        Credentials::Password(pass) => {
            call_contract_trusted_node(web3, Some(pass.as_str()), call_data, &eth)
        }
        Credentials::Secret(secret) => call_contract_local_sign(web3, &secret, call_data, &eth),
    }
}

/// Signs transaction with a secret key and sends a raw transaction to Ethereum node
fn call_contract_local_sign(
    web3: Web3<Http>,
    secret: &Secret,
    call_data: ethabi::Bytes,
    eth: &EthereumArgs,
) -> Result<H256, Error> {
    let gas_price = web3.eth().gas_price().wait().map_err(SyncFailure::new)?;
    let nonce = web3
        .eth()
        .transaction_count(eth.account, None)
        .wait()
        .map_err(SyncFailure::new)?;

    let tx = Transaction {
        nonce,
        value: "0".parse()?,
        action: Action::Call(eth.contract_address),
        data: call_data,
        gas: eth.gas.into(),
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
    web3: Web3<Http>,
    password: Option<&str>,
    call_data: ethabi::Bytes,
    eth: &EthereumArgs,
) -> Result<H256, Error> {
    let options = utils::options_with_gas(eth.gas);
    let tx_request = TransactionRequest {
        from: eth.account,
        to: Some(eth.contract_address),
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
    call_data: ethabi::Bytes,
    decoder: Box<R>,
    eth_url: &str,
    contract_address: Address,
) -> Result<R::Output, Error>
where
    R: ethabi::FunctionOutputDecoder,
{
    let (_eloop, transport) = web3::transports::Http::new(eth_url).map_err(SyncFailure::new)?;
    let web3 = web3::Web3::new(transport);
    let call_request = CallRequest {
        to: contract_address,
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

pub fn get_transaction_logs_raw<T, F>(
    eth_url: &str,
    tx: &H256,
    parse_log: F,
) -> Result<Vec<T>, Error>
where
    F: Fn(Log) -> Option<T>,
{
    let (_eloop, transport) = Http::new(eth_url).map_err(SyncFailure::new)?;
    let web3 = web3::Web3::new(transport);
    let receipt = web3
        .eth()
        .transaction_receipt(tx.clone())
        .wait()
        .map_err(SyncFailure::new)
        .context(format!(
            "Error retrieving transaction receipt for tx {:#x}",
            tx
        ))?
        .ok_or(err_msg(format!("No receipt for tx {:#x}", tx)))?;

    Ok(receipt.logs.into_iter().filter_map(parse_log).collect())
}

pub fn get_transaction_logs<T, F>(eth_url: &str, tx: &H256, parse_log: F) -> Result<Vec<T>, Error>
where
    F: Fn(RawLog) -> ethabi::Result<T>,
{
    get_transaction_logs_raw(eth_url, tx, |l| {
        let raw = RawLog::from((l.topics, l.data.0));
        parse_log(raw).ok()
    })
}

#[derive(Fail, Debug)]
#[fail(display = "TxError")]
pub enum TxError {
    #[fail(display = "No such transaction {:#x}", _0)]
    NoTx(H256),
    #[fail(display = "Undefined block number in transaction")]
    NoBlock,
    #[fail(display = "Web3 error {:?}", _0)]
    Web3Error(Error),
}

impl From<web3::Error> for TxError {
    fn from(e: web3::Error) -> TxError {
        TxError::Web3Error(SyncFailure::new(e).into())
    }
}

pub fn poll_get_transaction(eth_url: &str, tx: &H256) -> Result<Web3Transaction, Error> {
    use futures::future;
    use futures_retry::{FutureRetry, RetryPolicy};
    use tokio::runtime::Runtime;

    let mut rt = Runtime::new()?;

    let tx_cl = tx.clone();
    let eth_rul_cl = eth_url.to_string();
    let fut = FutureRetry::new(
        move || {
            let http_f = future::result(Http::new(eth_rul_cl.as_str()).map_err(|e| e.into()));
            http_f.and_then(|(_eloop, transport)| {
                let web3 = web3::Web3::new(transport);
                web3.eth()
                    .transaction(TransactionId::Hash(tx_cl))
                    .map_err(|e| e.into())
                    .and_then(|tx_res| tx_res.ok_or(TxError::NoTx(tx_cl)))
                    .and_then(|tx_res| tx_res.block_number.ok_or(TxError::NoBlock).and(Ok(tx_res)))
            })
        },
        |e| match e {
            TxError::NoTx(_) => RetryPolicy::ForwardError(e),
            TxError::NoBlock => {
                println!("No block");
                RetryPolicy::WaitRetry(Duration::from_millis(1000))
            }
            TxError::Web3Error(we) => {
                println!("Got web3 error {:?}", we);
                RetryPolicy::WaitRetry(Duration::from_millis(1000))
            }
        },
    );
    //    .wait()
    //    .map_err(|e| e.into())

    rt.block_on(fut).map_err(|e| e.into())
}
