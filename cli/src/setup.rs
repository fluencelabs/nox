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

use crate::config::none_if_empty;
use crate::config::SetupConfig;
use crate::config::HOME_DIR;
use crate::utils::parse_hex;
use clap::{App, AppSettings, SubCommand};
use failure::Error;
use rustyline::Editor;
use web3::types::Address;

pub fn interactive_setup(config: SetupConfig) -> Result<(), Error> {
    let mut rl = Editor::<()>::new();

    println!(
        "default contract address will be: {}",
        config.contract_address
    );
    println!("write contract address or press Enter for a default value:");
    let contract_address = rl.readline("> ")?;
    let contract_address = parse_hex(none_if_empty(contract_address.as_str()))?;
    let contract_address: Address = contract_address.unwrap_or(config.contract_address);
    println!("contract address is: {:?}", contract_address);

    println!("default ethereum node url will be: {}", config.eth_url);
    println!("write ethereum node url or press Enter for a default value");
    let ethereum_address = rl.readline("> ")?;
    let ethereum_address = none_if_empty(ethereum_address.as_str())
        .unwrap_or(config.eth_url.as_str())
        .to_owned();
    println!("ethereum node url is: {}", ethereum_address);

    println!("default swarm node url will be: {}", config.eth_url);
    println!("write swarm node url or press Enter for a default value");
    let swarm_address = rl.readline("> ")?;
    let swarm_address = none_if_empty(swarm_address.as_str())
        .unwrap_or(config.swarm_url.as_str())
        .to_owned();
    println!("swarm node url is: {}", swarm_address);

    println!("default account address will be: {:?}", config.account);
    println!("write account address or press Enter for a default value");
    let account_address = rl.readline("> ")?;
    let account_address: Option<Address> = parse_hex(none_if_empty(account_address.as_str()))?;
    let account_address = account_address.or_else(|| config.account);
    println!("account address is: {:?}", account_address);

    println!("default secret key will be: {:?}", config.secret_key);
    println!("write secret key or press Enter for a default value");
    let secret_key = rl.readline("> ")?;
    let secret_key = parse_hex(none_if_empty(secret_key.as_str()))?;
    println!("secret key is: {:?}", secret_key);

    println!(
        "default path to keystore will be: {:?}",
        config.keystore_path
    );
    println!("write path to keystore file or press Enter for a default value");
    let keystore_path = rl.readline("> ")?;
    let keystore_path = none_if_empty(keystore_path.as_str());
    println!("keystore path is: {:?}", keystore_path);

    println!(
        "default password for keystore will be: {:?}",
        config.password
    );
    println!("write password for keystore file or press Enter for a default value");
    let password = rl.readline("> ")?;
    let password = none_if_empty(password.as_str());
    println!("password is: {:?}", password);

    let config = SetupConfig::new(
        contract_address,
        account_address,
        ethereum_address,
        swarm_address,
        secret_key,
        keystore_path.map(|s| s.to_owned()),
        password.map(|s| s.to_owned()),
    );
    config.write_to_file(HOME_DIR)?;

    Ok(())
}

pub fn subcommand<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("setup")
        .about("Setup Fluence CLI with common parameters.")
        .unset_setting(AppSettings::ArgRequiredElseHelp)
        .unset_setting(AppSettings::SubcommandRequiredElseHelp)
}
