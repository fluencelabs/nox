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
use web3::types::{Address, Bytes, Transaction as Web3Transaction, H256};
use web3::types::{CallRequest, Log, TransactionId, TransactionRequest};
use web3::Web3;

use failure::{err_msg, Error, ResultExt, SyncFailure};

use crate::credentials::Credentials;
use crate::utils;
use ethabi::RawLog;
use std::time::Duration;

use_contract!(contract, "../bootstrap/contracts/compiled/Network.abi");

/// Calls contract method and returns hash of the transaction
pub fn call_contract(
    web3: &Web3<Http>,
    eth: &EthereumArgs,
    call_data: ethabi::Bytes,
) -> Result<H256, Error> {
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
    web3: &Web3<Http>,
    secret: &Secret,
    call_data: ethabi::Bytes,
    eth: &EthereumArgs,
) -> Result<H256, Error> {
    let nonce = web3
        .eth()
        .transaction_count(eth.account, None)
        .wait()
        .map_err(SyncFailure::new)?;

    let tx = Transaction {
        nonce: nonce,
        value: "0".parse()?,
        action: Action::Call(eth.contract_address),
        data: call_data,
        gas: eth.gas.into(),
        gas_price: eth.gas_price.into(),
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
    web3: &Web3<Http>,
    password: Option<&str>,
    call_data: ethabi::Bytes,
    eth: &EthereumArgs,
) -> Result<H256, Error> {
    let options = utils::options();
    let tx_request = TransactionRequest {
        from: eth.account,
        to: Some(eth.contract_address),
        data: Some(Bytes(call_data)),
        gas: Some(eth.gas.into()),
        gas_price: Some(eth.gas_price.into()),
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

// Load transaction receipt, read events (logs) from it, and parse them via `parse_log` closure
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

    receipt
        .status
        .filter(|s| !s.is_zero())
        .ok_or(err_msg(format!(
            "Transaction {:#x} has status of 0, meaning it was reverted",
            tx
        )))?;

    Ok(receipt.logs.into_iter().filter_map(parse_log).collect())
}

// Load transaction receipt, read events (logs) from it, and parse them via `parse_log` closure
pub fn get_transaction_logs<T, F>(eth_url: &str, tx: &H256, parse_log: F) -> Result<Vec<T>, Error>
where
    F: Fn(RawLog) -> ethabi::Result<T>,
{
    get_transaction_logs_raw(eth_url, tx, |l| {
        let raw = RawLog::from((l.topics, l.data.0));
        parse_log(raw).ok()
    })
}

// Block current thread and query `eth_getTransactionByHash` in loop, until transaction hash becomes non-null
pub fn wait_tx_included(tx: &H256, web3: &Web3<Http>) -> Result<Web3Transaction, Error> {
    use std::thread;

    // TODO: move to config or to options
    let max_attempts = 10;
    let mut attempt = 0;

    let tx = tx.clone();

    fn as_string(r: Result<Option<Web3Transaction>, Error>) -> String {
        match r {
            Err(e) => e.to_string(),
            _ => String::from("No transaction"),
        }
    }

    loop {
        let tx_opt: Result<Option<Web3Transaction>, Error> = web3
            .eth()
            .transaction(TransactionId::Hash(tx))
            .map_err(SyncFailure::new)
            .map_err(Into::into)
            .wait();

        attempt += 1;

        match tx_opt {
            Ok(Some(web3tx)) => {
                if web3tx.block_number.is_some() {
                    return Ok(web3tx);
                }
            }
            r => {
                if attempt == max_attempts {
                    return Err(err_msg(format!(
                        "[{:?}] All attempts ({}) have been used",
                        as_string(r),
                        max_attempts
                    )));
                } else {
                    eprintln!(
                        "[{:?}] Attempt {}/{} has failed",
                        as_string(r),
                        attempt,
                        max_attempts
                    )
                }
            }
        }

        // TODO: move to config or to options
        thread::sleep(Duration::from_secs(5));
    }
}

// Block current thread and query Ethereum node with eth_syncing until it finishes syncing blocks
pub fn wait_sync(web3: &Web3<Http>) -> Result<(), Error> {
    use std::thread;

    // TODO: move to config or to options
    let ten_seconds = Duration::from_secs(10);

    loop {
        if !utils::check_sync(web3)? {
            break;
        }

        thread::sleep(ten_seconds);
    }

    Ok(())
}
