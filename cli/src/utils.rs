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

use std::error::Error;

use clap::ArgMatches;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Url, UrlError};
use web3::contract::Options;
use web3::futures::Future;
use web3::transports::Http;
use web3::types::SyncState;
use web3::Web3;
use ethkey::Secret;

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

/// Parses URL from the string
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

pub fn check_sync(web3: &Web3<Http>) -> Result<bool, Box<Error>> {
    let sync_state = web3.eth().syncing().wait()?;
    match sync_state {
        SyncState::Syncing(_) => Ok(true),
        SyncState::NotSyncing => Ok(false),
    }
}

/// Creates options for transaction to ethereum
pub fn options_with_gas(gas_limit: u32) -> Options {
    Options::with(|default| {
        default.gas = Some(gas_limit.into());
    })
}

#[allow(unused)]
pub fn options() -> Options {
    Options::default()
}

/// Gets the value of option `key` and removes '0x' prefix
pub fn parse_hex_opt(matches: &ArgMatches, key: &str) -> Result<String, Box<Error>> {
    Ok(value_t!(matches, key, String)?.trim_start_matches("0x").to_string())
}

pub fn parse_secret_key(matches: &ArgMatches, key: &str) -> Result<Option<Secret>, Box<Error>> {
    matches
        .value_of(SECRET_KEY)
        .map(|s| s.trim_start_matches("0x").parse::<Secret>())
        .map_or(Ok(None), |r| r.map(Some))? // Option<Result> -> Result<Option>
}