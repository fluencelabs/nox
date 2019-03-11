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
use std::fmt::Display;

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

fn format_option<T>(opt: &Option<T>) -> String
    where
        T: Debug,
{
    match opt {
        Some(v) => format!("{:?}", v),
        None => "empty".to_owned(),
    }
}

pub fn interactive_setup(config: SetupConfig) -> Result<(), Error> {
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
    let swarm_url = none_if_empty(&swarm_address)
        .unwrap_or(&config.swarm_url)
        .trim()
        .to_owned();

    let auth = ask_auth(&mut rl)?;
    let credentials = match auth {
        Auth::SecretKey => {
            let saved_key = get_saved_key(&config);
            secret_key_auth(&mut rl, saved_key)
        }
        Auth::Keystore => {
            let (saved_path, saved_password) = get_saved_keystore(&config);
            keystore_auth(&mut rl, saved_path, saved_password)
        }
        Auth::None => Credentials::No,
    };

    let config = SetupConfig::new(contract_address, ethereum_url, swarm_url, credentials);
    config.write_to_file()?;

    Ok(())
}

fn get_saved_key(config: &SetupConfig) -> Option<Secret> {
    if let Credentials::Secret(ref old_key) = config.credentials {
        Some(old_key.clone())
    } else {
        None
    }
}

fn get_saved_keystore(config: &SetupConfig) -> (Option<String>, Option<String>) {
    if let Credentials::Keystore {
        secret: _,
        ref path,
        ref password,
    } = config.credentials
    {
        (Some(path.clone()), Some(password.clone()))
    } else {
        (None, None)
    }
}

fn ask_auth(rl: &mut Editor<()>) -> Result<Auth, Error> {
    let auth_prompt = format!("Choose authorization type. (K)eystore / (S)ecret key / (N)one [K]:");
    let auth = loop {
        let auth = rl.readline(&auth_prompt)?;
        let auth = auth.to_lowercase();
        if auth.starts_with("k") || auth.is_empty() {
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

fn report(err: Error) {
    println!("Error occured. {}", err);
    let fail = err.as_fail();
    for cause in fail.iter_causes() {
        println!("Caused by: {}", cause);
    }
}

fn secret_key_auth(rl: &mut Editor<()>, saved_key: Option<Secret>) -> Credentials {
    let secret_key_prompt = format!("Secret Key [{}]: ", format_option(&saved_key));
    fn load(
        rl: &mut Editor<()>,
        saved_key: Option<Secret>,
        prompt: &String,
    ) -> Result<Credentials, Error> {
        let secret_key = rl.readline(prompt).expect("Interruped."); //TODO: remove expect, use ?
        let secret_key = parse_hex(none_if_empty(&secret_key))?;
        let secret = secret_key
            .or(saved_key)
            .ok_or(err_msg("Secret Key cannot be empty"))?;

        Credentials::from_secret(secret)
    };

    loop {
        match load(rl, saved_key.clone(), &secret_key_prompt) {
            Ok(c) => break c,
            Err(e) => report(e),
        }
    }
}

fn keystore_auth(
    rl: &mut Editor<()>,
    saved_path: Option<String>,
    saved_password: Option<String>,
) -> Credentials {
    let keystore_path_prompt = format!("Keystore Path [{}]: ", format_option(&saved_path));
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
            let keystore_path = rl.readline(&keystore_path_prompt).expect("Interrupted."); //TODO: remove expect, use ?
            let keystore_path = none_if_empty_string(keystore_path)
                .or(saved_path.clone())
                .ok_or(err_msg("Keystore Path cannot be empty"));
            match keystore_path {
                Ok(kp) => break kp,
                Err(e) => report(e),
            }
        };

        let password = loop {
            let password = rl.readline(&password_prompt).expect("Interrupted"); //TODO: remove expect, use ?
            let password = none_if_empty_string(password)
                .or(saved_password.clone())
                .ok_or(err_msg("Password cannot be empty"));
            match password {
                Ok(p) => break p,
                Err(e) => report(e),
            }
        };

        match Credentials::load_keystore(keystore_path.clone(), password) {
            Ok(c) => break c,
            Err(e) => {
                println!("Error while loading keystore at {}", keystore_path.clone());
                report(e);
//                println!("Password and/or path are incorrect");
            }
        }
    }
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("setup")
        .about("Setup Fluence CLI with common parameters.")
        .unset_setting(AppSettings::ArgRequiredElseHelp)
        .unset_setting(AppSettings::SubcommandRequiredElseHelp)
}
