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

use exitfailure::ExitFailure;
use failure::{err_msg, ResultExt};
use fluence::config::SetupConfig;
use fluence::config::HOME_DIR;
use fluence::publisher::Published;
use fluence::register::Registered;
use fluence::utils;
use fluence::{
    check, contract_status, delete_all, delete_app, delete_node, publisher, register, setup,
};
use web3::types::H256;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), ExitFailure> {
    let config = SetupConfig::read_from_file(HOME_DIR)?;

    let app = App::new("Fluence CLI")
        .global_setting(AppSettings::UnifiedHelpMessage)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .version(VERSION)
        .author("Fluence Labs")
        .about("Console utility for deploying code to fluence cluster")
        .subcommand(publisher::subcommand())
        .subcommand(register::subcommand())
        .subcommand(contract_status::subcommand())
        .subcommand(check::subcommand())
        .subcommand(delete_app::subcommand())
        .subcommand(delete_node::subcommand())
        .subcommand(delete_all::subcommand())
        .subcommand(setup::subcommand());

    match app.get_matches().subcommand() {
        ("publish", Some(args)) => {
            let publisher = publisher::parse(args, config).context("Error parsing arguments")?;
            let published = publisher
                .publish(true)
                .context("Error sending transaction")?;

            let print_status = |app_id: u64, tx: H256, status: &str| {
                println!("{}", style(format!("App {}.", status)).blue());
                utils::print_info_msg("app id:", app_id.to_string());
                utils::print_tx_hash(tx);
            };

            match published {
                Published::Deployed { app_id, tx } => print_status(app_id, tx, "deployed"),
                Published::Enqueued { app_id, tx } => print_status(app_id, tx, "enqueued"),
                Published::TransactionSent(tx) => utils::print_tx_hash(tx),
            }
        }

        ("register", Some(args)) => {
            let register = register::parse(args, config).context("Error parsing arguments")?;
            let registered = register
                .register(true)
                .context("Error sending transaction")?;

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
                Registered::AlreadyRegistered => println!(
                    "{}",
                    style(format!("The node has already been registered.")).blue()
                ),
            }
        }

        ("status", Some(args)) => {
            let status = contract_status::get_status_by_args(args, config)
                .context("Error retrieving status")?;

            if let Some(status) = status {
                let json = serde_json::to_string_pretty(&status)
                    .context("Error serializing status to pretty JSON")?;

                println!("{}", json);
            }
        }

        ("check", Some(args)) => check::process(args)?,

        ("delete_app", Some(args)) => {
            let delete_app = delete_app::parse(args, config).context("Error parsing arguments")?;
            let tx: H256 = delete_app
                .delete_app(true)
                .context("Error sending transaction")?;

            utils::print_info_id("App deleted. Submitted transaction", tx);
        }

        ("delete_node", Some(args)) => {
            let delete_node =
                delete_node::parse(args, config).context("Error parsing arguments")?;
            let tx: H256 = delete_node
                .delete_node(true)
                .context("Error sending transaction")?;

            utils::print_info_id("Node deleted. Submitted transaction", tx);
        }

        ("delete_all", Some(args)) => {
            let delete_all = delete_all::parse(args, config).context("Error parsing arguments")?;
            delete_all
                .delete_all()
                .context("Error sending transaction")?;

            println!("All nodes and apps should be deleted. Check contract with `status` command.");
        }

        ("setup", _) => {
            setup::interactive_setup().context("Error making setup of Fluence CLI")?;

            println!("Setup completed.");
        }

        c => Err(err_msg(format!("Unexpected command: {}", c.0)))?,
    }

    Ok(())
}
