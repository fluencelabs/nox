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
use ethkey::Secret;
use failure::{err_msg, Error};
use rustyline::Editor;

use crate::config::none_if_empty;
use crate::config::none_if_empty_string;
use crate::config::Auth;
use crate::config::SetupConfig;
use crate::credentials::Credentials;
use crate::utils::parse_hex;

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("setup")
        .about("Setup Fluence CLI with common parameters.")
        .unset_setting(AppSettings::ArgRequiredElseHelp)
        .unset_setting(AppSettings::SubcommandRequiredElseHelp)
}

pub fn interactive_setup(config: SetupConfig) -> Result<(), Error> {
    let SetupConfig {
        contract_address: cfg_contract_address,
        eth_url: cfg_eth_url,
        swarm_url: cfg_swarm_url,
        credentials: cfg_credentials,
    } = config;

    let rl = &mut Editor::<()>::new();

    let contract_address_prompt = format!("Contract Address [{:?}]: ", cfg_contract_address);
    let contract_address = loop {
        let contract_address = readline(rl, &contract_address_prompt)?;
        let contract_address = parse_hex(none_if_empty(&contract_address));
        match contract_address {
            Ok(r) => break r.unwrap_or(cfg_contract_address),
            Err(e) => report(e.into()),
        }
    };

    let ethereum_url_prompt = format!("Ethereum Node Url [{}]: ", cfg_eth_url);
    let ethereum_address = readline(rl, &ethereum_url_prompt)?;
    let ethereum_url = none_if_empty_string(ethereum_address).unwrap_or(cfg_eth_url);

    let swarm_url_prompt = format!("Swarm Node Url [{}]: ", cfg_swarm_url);
    let swarm_address = readline(rl, &swarm_url_prompt)?;
    let swarm_url = none_if_empty_string(swarm_address).unwrap_or(cfg_swarm_url);

    let auth = ask_auth(rl, &cfg_credentials)?;
    let credentials = match auth {
        Auth::SecretKey => {
            let saved_key = get_saved_key(&cfg_credentials);
            secret_key_auth(rl, saved_key)?
        }
        Auth::Keystore => {
            let (saved_path, saved_password) = get_saved_keystore(&cfg_credentials);
            keystore_auth(rl, saved_path, saved_password)?
        }
        Auth::None => Credentials::No,
    };

    let config = SetupConfig::new(contract_address, ethereum_url, swarm_url, credentials);
    config.write_to_file()?;

    Ok(())
}

fn get_saved_key(credentials: &Credentials) -> Option<&Secret> {
    if let Credentials::Secret(ref old_key) = credentials {
        Some(old_key)
    } else {
        None
    }
}

fn get_saved_keystore(credentials: &Credentials) -> (Option<&String>, Option<&String>) {
    if let Credentials::Keystore {
        secret: _,
        ref path,
        ref password,
    } = credentials
    {
        (Some(path), Some(password))
    } else {
        (None, None)
    }
}

fn get_saved_auth(credentials: &Credentials) -> Auth {
    match credentials {
        Credentials::No => Auth::None,
        Credentials::Secret(_) => Auth::SecretKey,
        Credentials::Keystore { .. } => Auth::Keystore,
    }
}

fn ask_auth(rl: &mut Editor<()>, credentials: &Credentials) -> Result<Auth, Error> {
    let saved_auth = get_saved_auth(credentials);
    let saved_letter = saved_auth.first_letter();
    println!("Choose Ethereum authorization method.");
    let auth_prompt = format!("(K)eystore / (S)ecret key / (N)one [{}]: ", saved_letter);
    let auth = loop {
        let auth = readline(rl, &auth_prompt)?;
        let auth = auth.to_lowercase();
        if auth.is_empty() {
            break saved_auth;
        } else if auth.starts_with("k") {
            break Auth::Keystore;
        } else if auth.starts_with("s") {
            break Auth::SecretKey;
        } else if auth.starts_with("n") {
            break Auth::None;
        } else {
            println!(
                "Please, enter K - for Keystore, S - for Secret key, or N - for no authorization"
            )
        }
    };

    Ok(auth)
}

fn secret_key_auth(rl: &mut Editor<()>, saved_key: Option<&Secret>) -> Result<Credentials, Error> {
    let secret_key_prompt = format!(
        "Ethereum account secret key [{}]: ",
        format_option(&saved_key)
    );
    fn load(
        rl: &mut Editor<()>,
        saved_key: Option<&Secret>,
        prompt: &String,
    ) -> Result<Credentials, Error> {
        let secret_key = readline(rl, prompt)?;
        let secret_key = parse_hex(none_if_empty(&secret_key))?;
        let secret = secret_key
            .or(saved_key.map(|s| s.clone()))
            .ok_or(err_msg("Secret Key cannot be empty"))?;

        Credentials::from_secret(secret)
    };

    loop {
        match load(rl, saved_key, &secret_key_prompt) {
            ok @ Ok(_) => break ok,
            Err(e) => report(e),
        }
    }
}

fn keystore_auth(
    rl: &mut Editor<()>,
    saved_path: Option<&String>,
    saved_password: Option<&String>,
) -> Result<Credentials, Error> {
    let keystore_path_prompt = format!(
        "Ethereum account keystore path [{}]: ",
        format_option(&saved_path)
    );
    let password_prompt = format!(
        "Password [{}]: ",
        if saved_password.is_some() {
            "*******"
        } else {
            "empty"
        }
    );

    loop {
        let keystore_path = loop {
            let keystore_path = readline(rl, &keystore_path_prompt)?;
            let keystore_path = none_if_empty_string(keystore_path)
                .or(saved_path.map(|s| s.clone()))
                .ok_or(err_msg("Keystore Path cannot be empty"));
            match keystore_path {
                Ok(kp) => break kp,
                Err(e) => report(e),
            }
        };

        let password = loop {
            let password = readline(rl, &password_prompt)?;
            let password = none_if_empty_string(password)
                .or(saved_password.map(|s| s.clone()))
                .ok_or(err_msg("Password cannot be empty"));
            match password {
                Ok(p) => break p,
                Err(e) => report(e),
            }
        };

        match Credentials::load_keystore(keystore_path.clone(), password) {
            ok @ Ok(_) => break ok,
            Err(e) => {
                println!("Error while loading keystore. Path: `{}`", keystore_path);
                report(e);
            }
        }
    }
}

fn readline(rl: &mut Editor<()>, prompt: &str) -> Result<String, Error> {
    rl.readline(prompt)
        .map(|s| s.trim().to_string())
        .map_err(Into::into)
}

fn report(err: Error) {
    println!("Error occured. {}", err);
    let fail = err.as_fail();
    for cause in fail.iter_causes() {
        println!("Caused by: {}", cause);
    }
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
