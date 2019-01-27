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

use clap::App;
use clap::AppSettings;
use console::style;

use fluence::contract_status::ui::rich_status;
use fluence::publisher::Published;
use fluence::register::Registered;
use fluence::utils;
use fluence::{check, contract_status, delete_app, delete_node, publisher, register};
use web3::types::H256;

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
        .subcommand(check::subcommand())
        .subcommand(delete_app::subcommand())
        .subcommand(delete_node::subcommand());

    match app.get_matches().subcommand() {
        ("publish", Some(args)) => {
            let publisher = publisher::parse(args).expect("Error parsing arguments");
            let published = publisher.publish(true).expect("Error sending transaction");

            let print_status = |app_id: H256, tx: H256, status: &str| {
                println!("{}", style(format!("App {}.", status)).blue());
                utils::print_info_id_short("app id:", app_id);
                utils::print_tx_hash(tx);
            };

            match published {
                Published::Deployed { app_id, tx } => print_status(app_id, tx, "deployed"),
                Published::Enqueued { app_id, tx } => print_status(app_id, tx, "enqueued"),
                Published::TransactionSent(tx) => utils::print_tx_hash(tx),
            }
        }

        ("register", Some(args)) => {
            let register = register::parse(args).expect("Error parsing arguments");
            let registered = register.register(true).expect("Error sending transaction");

            match registered {
                Registered::Deployed { app_ids, ports, tx } => {
                    println!("{}", style(format!("Node deployed.")).blue());

                    for (app_id, port) in app_ids.iter().zip(ports.iter()) {
                        println!(
                            "{0: >10} {1: ^10} {2: >10} {3:#x}",
                            style("port:").blue(),
                            style(port).red().bold(),
                            style("app id:").blue(),
                            style(app_id).red().bold()
                        );
                    }
                    utils::print_tx_hash(tx);
                }
                Registered::Enqueued(tx) => {
                    println!("{}", style(format!("Node registered.")).blue());
                    utils::print_tx_hash(tx);
                }
                Registered::TransactionSent(tx) => utils::print_tx_hash(tx),
            }
        }

        ("status", Some(args)) => {
            let status = contract_status::get_status_by_args(args).unwrap();

            let json = serde_json::to_string_pretty(&status).unwrap();

            println!("{}", json);
        }

        ("check", Some(args)) => {
            handle_error(check::process(args));
        }

        ("delete_app", Some(args)) => {
            let delete_app = delete_app::parse(args).expect("Error parsing arguments");
            let tx: H256 = delete_app
                .delete_app(true)
                .expect("Error sending transaction");

            utils::print_info_id("App deleted. Submitted transaction", tx);
        }

        ("delete_node", Some(args)) => {
            let delete_node = delete_node::parse(args).expect("Error parsing arguments");
            let tx = delete_node
                .delete_node(true)
                .expect("Error sending transaction");

            utils::print_info_id("Node deleted. Submitted transaction", tx);
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
