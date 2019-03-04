use crate::config::CliConfig;
use crate::config::HOME_DIR;
use clap::{value_t, App, AppSettings, Arg, ArgMatches, SubCommand};
use ethkey::Secret;
use failure::Error;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use crate::utils::parse_h256;
use web3::types::Address;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json;

fn empty_or_default(value: String, default: String) -> String {
    if (value.is_empty()) {
        default
    } else {
        value
    }
}

fn none_if_empty(value: &str) -> Option<&str> {
    if (value.is_empty()) {
        None
    } else {
        Some(value)
    }
}

pub fn interactive_setup() -> Result<(), Error> {
    let mut rl = Editor::<()>::new();
    println!("write your contract address...");
    let contract_address = rl.readline("> ")?;
    let contract_address: Address = contract_address.as_str().trim_left_matches("0x").parse()?;
    println!("your contract address is: {}", contract_address);

    println!("write ethereum address or press enter for default");
    let ethereum_address = rl.readline("> ")?;
    let ethereum_address =
        empty_or_default(ethereum_address, "http://data.fluence.ai:8545/".to_owned());
    println!("your ethereum address is: {}", ethereum_address);

    println!("write swarm address or press enter for default");
    let swarm_address = rl.readline("> ")?;
    let swarm_address = empty_or_default(swarm_address, "http://data.fluence.ai:8500/".to_owned());
    println!("your swarm address is: {}", swarm_address);

    println!("write your account address...");
    let account_address = rl.readline("> ")?;
    let account_address: Address = account_address.as_str().trim_left_matches("0x").parse()?;
    println!("your account address is: {}", account_address);

    println!("write your secret key, press Enter if empty");
    let secret_key = rl.readline("> ")?;
    let secret_key = parse_h256(none_if_empty(secret_key.as_str()))?;
    println!("your secret key is: {:?}", secret_key);

    println!("write your path to keystore file, press Enter if empty");
    let keystore_path = rl.readline("> ")?;
    let keystore_path = none_if_empty(keystore_path.as_str());
    println!("your keystore path is: {:?}", keystore_path);

    println!("write your password for keystore file, press Enter if empty");
    let password = rl.readline("> ")?;
    let password = none_if_empty(password.as_str());
    println!("your password is: {:?}", password);

    let config = CliConfig::new(
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
