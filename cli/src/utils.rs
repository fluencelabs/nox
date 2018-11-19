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

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Url, UrlError};
use std::error::Error;
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::contract::{Contract, Options};
use web3::futures::Future;
use web3::types::{Address, H256};
use web3::{Transport, Web3};

/// Creates progress bar in the console until the work is over
///
/// # Arguments
///
/// * `msg` - message on progress bar while working in progress
/// * `prefix`
/// * `finish` - message after work is done
/// * `work` - some function to be done
///
/// # Examples
/// ```
/// with_progress("Code uploading...", "1/2", "Code uploaded.", upload_fn)
/// ```
/// The output while processing:
/// ```
/// [1/2] ⠙ Code uploading... ---> [00:00:05]
/// ```
/// The output on the finish:
/// ```
/// [1/2]   Code uploaded. ---> [00:00:10]
/// ```
///
pub fn with_progress<U, F: FnOnce() -> U>(msg: &str, prefix: &str, finish: &str, work: F) -> U {
    let bar = create_progress_bar(prefix, msg);
    let result = work();
    bar.finish_with_message(finish);
    result
}

const TEMPLATE: &str = "[{prefix:.blue}] {spinner} {msg:.blue} ---> [{elapsed_precise:.blue}]";

/// Creates a spinner progress bar, that will be tick at once
fn create_progress_bar(prefix: &str, msg: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_message(msg);
    bar.set_prefix(prefix);
    bar.enable_steady_tick(100);
    bar.set_style(ProgressStyle::default_spinner().template(TEMPLATE));

    bar
}

fn get_contract<T: Transport>(
    web3: Web3<T>,
    contract_address: Address,
) -> Result<Contract<T>, Box<Error>> {
    let json = include_bytes!("../../bootstrap/contracts/compiled/Deployer.abi");

    Ok(Contract::from_json(web3.eth(), contract_address, json)?)
}

/// Publishes hash of the code (address in swarm) to the `Deployer` smart contract
pub fn call_contract<P>(
    account: Address,
    contract_address: Address,
    password: Option<&str>,
    eth_url: &str,
    func: &str,
    params: P,
    options: Options,
) -> Result<H256, Box<Error>>
where
    P: Tokenize,
{
    let (_eloop, transport) = web3::transports::Http::new(&eth_url)?;
    let web3 = web3::Web3::new(transport);

    if let Some(p) = password {
        web3.personal().unlock_account(account, p, None).wait()?;
    }

    let contract = get_contract(web3, contract_address)?;

    let result_code_publish = contract.call(func, params, account, options);
    Ok(result_code_publish.wait()?)
}

pub fn query_contract<P, R>(
    contract_address: Address,
    eth_url: &str,
    func: &str,
    params: P,
    options: Options,
) -> Result<R, Box<Error>>
where
    P: Tokenize,
    R: Detokenize,
{
    let (_eloop, transport) = web3::transports::Http::new(&eth_url)?;
    let web3 = web3::Web3::new(transport);

    let contract = get_contract(web3, contract_address)?;

    let result_code_publish = contract.query(func, params, None, options, None);
    let res = result_code_publish.wait()?;
    Ok(res)
}

pub fn add_to_white_list(
    eth_url: &str,
    account_to_add: Address,
    contract_address: Address,
    owner: Address,
) -> Result<H256, Box<Error>> {
    let (_eloop, transport) = web3::transports::Http::new(eth_url)?;
    let web3 = web3::Web3::new(transport);

    let contract = get_contract(web3, contract_address)?;

    Ok(contract
        .call(
            "addAddressToWhitelist",
            account_to_add,
            owner,
            Options::default(),
        ).wait()?)
}

pub fn parse_url(url: &str) -> Result<Url, UrlError> {
    match Url::parse(url) {
        Ok(url) => Ok(url),
        Err(error) if error == UrlError::RelativeUrlWithoutBase => {
            let url_with_base = format!("http://{}", url);
            Url::parse(url_with_base.as_str())
        }
        Err(error) => Err(error),
    }
}
