use crate::config::empty_or_default;
use crate::config::none_if_empty;
use crate::config::SetupConfig;
use crate::config::DEFAULT_CONTRACT_ADDRESS;
use crate::config::DEFAULT_ETH_URL;
use crate::config::DEFAULT_SWARM_URL;
use crate::config::HOME_DIR;
use crate::utils::parse_hex;
use clap::{App, AppSettings, SubCommand};
use failure::Error;
use rustyline::Editor;
use web3::types::Address;

pub fn interactive_setup() -> Result<(), Error> {
    let mut rl = Editor::<()>::new();
    println!("write your contract address...");
    let contract_address = rl.readline("> ")?;
    let contract_address: Address =
        empty_or_default(contract_address, DEFAULT_CONTRACT_ADDRESS.to_owned())
            .trim_start_matches("0x")
            .parse()?;
    println!("your contract address is: {:?}", contract_address);

    println!("write ethereum address or press enter for default");
    let ethereum_address = rl.readline("> ")?;
    let ethereum_address = empty_or_default(ethereum_address, DEFAULT_ETH_URL.to_owned());
    println!("your ethereum address is: {}", ethereum_address);

    println!("write swarm address or press enter for default");
    let swarm_address = rl.readline("> ")?;
    let swarm_address = empty_or_default(swarm_address, DEFAULT_SWARM_URL.to_owned());
    println!("your swarm address is: {}", swarm_address);

    println!("write your account address...");
    let account_address = rl.readline("> ")?;
    let account_address = parse_hex(none_if_empty(account_address.as_str()))?;
    println!("your account address is: {:?}", account_address);

    println!("write your secret key, press Enter if empty");
    let secret_key = rl.readline("> ")?;
    let secret_key = parse_hex(none_if_empty(secret_key.as_str()))?;
    println!("your secret key is: {:?}", secret_key);

    println!("write your path to keystore file, press Enter if empty");
    let keystore_path = rl.readline("> ")?;
    let keystore_path = none_if_empty(keystore_path.as_str());
    println!("your keystore path is: {:?}", keystore_path);

    println!("write your password for keystore file, press Enter if empty");
    let password = rl.readline("> ")?;
    let password = none_if_empty(password.as_str());
    println!("your password is: {:?}", password);

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
