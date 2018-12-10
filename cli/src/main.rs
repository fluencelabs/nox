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
extern crate hex;
extern crate indicatif;
extern crate reqwest;
extern crate web3;
#[macro_use]
extern crate fixed_hash;
#[macro_use]
extern crate error_chain;

extern crate core;
extern crate parity_wasm;
#[cfg(test)]
extern crate rand;

mod check;
mod publisher;
mod register;
mod status;
mod types;
mod utils;
mod whitelist;

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
        .subcommand(status::subcommand())
        .subcommand(whitelist::subcommand())
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

        ("add-to-whitelist", Some(args)) => {
            let add_to_whitelist = whitelist::parse(args).unwrap();

            let transaction = add_to_whitelist.add_to_whitelist(true);

            let formatted_finish_msg =
                style("Address added to whitelist. Submitted transaction").blue();
            let formatted_tx = style(transaction.unwrap()).red().bold();

            println!("{}: {:?}", formatted_finish_msg, formatted_tx);
        }

        ("status", Some(args)) => {
            let status = status::get_status_by_args(args).unwrap();

            //          println!("Status of Fluence smart contract:\n{}", status);
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
