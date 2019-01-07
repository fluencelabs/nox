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
extern crate clap;
extern crate console;
extern crate ethabi;
extern crate ethkey;
extern crate hex;
extern crate indicatif;
extern crate reqwest;
extern crate web3;
#[macro_use]
extern crate fixed_hash;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate derive_getters;

extern crate base64;
extern crate common_types;
extern crate rlp;

extern crate ethereum_types_serialize;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate core;
extern crate parity_wasm;
#[cfg(test)]
extern crate rand;

#[macro_use]
extern crate ethabi_contract;

#[macro_use]
extern crate ethabi_derive;

mod check;
mod contract_func;
mod contract_status;
mod credentials;
mod publisher;
mod register;
mod types;
mod utils;

use clap::App;
use clap::AppSettings;
use console::style;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let app = App::new("Fluence CLI")
        .setting(AppSettings::ArgRequiredElseHelp)
        .version(VERSION)
        .author("Fluence Labs")
        .about("Console utility for deploying code to fluence cluster")
        .subcommand(publisher::subcommand())
        .subcommand(register::subcommand())
        .subcommand(contract_status::subcommand())
        .subcommand(check::subcommand());

    match app.get_matches().subcommand() {
        ("publish", Some(args)) => {
            let publisher = publisher::parse(args).unwrap();

            let transaction = publisher.publish(true);

            let formatted_finish_msg = style("Code published. Submitted transaction").blue();
            let formatted_tx = style(transaction.unwrap()).red().bold();

            println!("{}: {:?}", formatted_finish_msg, formatted_tx);
        }

        ("register", Some(args)) => {
            let register = register::parse(args).unwrap();

            let transaction = register.register(true);

            let formatted_finish_msg = style("Solver added. Submitted transaction").blue();
            let formatted_tx = style(transaction.unwrap()).red().bold();

            println!("{}: {:?}", formatted_finish_msg, formatted_tx);
        }

        ("status", Some(args)) => {
            let status = contract_status::get_status_by_args(args).unwrap();

            let json = serde_json::to_string_pretty(&status).unwrap();

            println!("{}", json);
        }

        ("check", Some(args)) => {
            handle_error(check::process(args));
        }

        c => panic!("Unexpected command: {}", c.0),
    }
}

fn handle_error<T, E>(result: Result<T, E>)
where
    E: error_chain::ChainedError,
{
    if let Err(err) = result {
        use std::io::Write;

        let stderr = &mut ::std::io::stderr();
        writeln!(stderr, "{}", err.display_chain()).expect("Error writing to stderr");
        ::std::process::exit(1);
    }
}
