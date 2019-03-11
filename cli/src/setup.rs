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

use std::fmt::Debug;

use clap::{App, AppSettings, SubCommand};
use failure::Error;
use rustyline::Editor;

use crate::config::none_if_empty;
use crate::config::none_if_empty_string;
use crate::config::SetupConfig;
use crate::utils::parse_hex;
use web3::types::Address;
use web3::types::H256;

enum Auth {
    SecretKey,
    Keystore,
}

fn format_option<T>(opt: &Option<T>) -> String
    where
        T: Debug,
{
    match opt {
        Some(v) => format!("{:?}", v),
        None => "empty".to_owned(),
    }
}

pub fn interactive_setup(config: &SetupConfig) -> Result<(), Error> {
    let mut rl = Editor::<()>::new();

    let contract_address_prompt = format!("Contract Address [{:?}]: ", config.contract_address);
    let contract_address = loop {
        let contract_address = rl.readline(&contract_address_prompt)?;
        let contract_address = parse_hex(
            none_if_empty_string(contract_address)
                .as_ref()
                .map(|s| s.as_str()),
        );
        match contract_address {
            Ok(r) => break r.unwrap_or(config.contract_address),
            Err(e) => {
                println!("error occured {}", e);
                println!("try again");
            }
        }
    };

    let ethereum_url_prompt = format!("Ethereum Node Url [{}]: ", config.eth_url);
    let ethereum_address = rl.readline(&ethereum_url_prompt)?;
    let ethereum_url = none_if_empty(&ethereum_address)
        .unwrap_or(&config.eth_url)
        .trim()
        .to_owned();

    let swarm_url_prompt = format!("Swarm Node Url [{}]: ", config.swarm_url);
    let swarm_address = rl.readline(&swarm_url_prompt)?;
    let swarm_address = none_if_empty(&swarm_address)
        .unwrap_or(&config.swarm_url)
        .trim()
        .to_owned();

    let auth_prompt = format!("Choose authorization type. (K)eystore / (S)ecret key [K]:");
    let auth = loop {
        let auth = rl.readline_with_initial(&auth_prompt, ("left", "right"))?;
        let auth = auth.to_lowercase();
        if auth.starts_with("k") || auth.is_empty() {
            break Auth::Keystore;
        } else if auth.starts_with("s") {
            break Auth::SecretKey;
        } else {
            println!("Please, enter K for Keystore or S for Secret key")
        }
    };

    let config = match auth {
        Auth::SecretKey => {
            let (account_address, secret_key) = secret_key_auth(&mut rl, config)?;
            SetupConfig::new(
                contract_address,
                account_address,
                ethereum_url,
                swarm_address,
                secret_key,
                None,
                None,
            )
        }
        Auth::Keystore => {
            let (keystore_path, password) = keystore_auth(&mut rl, config)?;
            SetupConfig::new(
                contract_address,
                None,
                ethereum_url,
                swarm_address,
                None,
                keystore_path.map(|s| s.to_owned()),
                password.map(|s| s.to_owned()),
            )
        }
    };

    config.write_to_file()?;
    Ok(())
}

fn secret_key_auth(
    rl: &mut Editor<()>,
    config: &SetupConfig,
) -> Result<(Option<Address>, Option<H256>), Error> {
    let account_address_prompt = format!("Account Address [{}]: ", format_option(&config.account));
    let account_address = loop {
        let account_address = rl.readline(&account_address_prompt)?;
        match parse_hex(none_if_empty(&account_address)) {
            Ok(r) => break r.or_else(|| config.account),
            Err(e) => {
                println!("error occured {}", e);
                println!("try again");
            }
        }
    };

    let secret_key_prompt = format!("Secret Key [{}]: ", format_option(&config.secret_key));
    let secret_key = loop {
        let secret_key = rl.readline(&secret_key_prompt)?;
        match parse_hex(none_if_empty(&secret_key)) {
            Ok(r) => break r.or_else(|| config.secret_key),
            Err(e) => {
                println!("error occured {}", e);
                println!("try again");
            }
        };
    };

    Ok((account_address, secret_key))
}

fn keystore_auth(
    rl: &mut Editor<()>,
    config: &SetupConfig,
) -> Result<(Option<String>, Option<String>), Error> {
    let keystore_path_prompt =
        format!("Keystore Path [{}]: ", format_option(&config.keystore_path));
    let keystore_path = rl.readline(&keystore_path_prompt)?;
    let keystore_path = none_if_empty_string(keystore_path).or(config.keystore_path.clone());

    let password_prompt = format!("Password [{}]: ", format_option(&config.password));
    let password = rl.readline(&password_prompt)?;
    let password = none_if_empty_string(password).or(config.password.clone());

    Ok((keystore_path, password))
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("setup")
        .about("Setup Fluence CLI with common parameters.")
        .unset_setting(AppSettings::ArgRequiredElseHelp)
        .unset_setting(AppSettings::SubcommandRequiredElseHelp)
}
