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
extern crate web3;

pub mod app {

    use clap::{App, Arg};
    use console::style;
    use std::boxed::Box;
    use web3::types::Address;

    pub struct Publisher {
        pub path: String,
        pub contract_address: Address,
        pub account: Address,
        pub swarm_url: String,
        pub eth_url: String,
        pub password: Option<String>,
        pub cluster_size: u64,
    }

    /// Parses arguments from console and initialize parameters for Publisher
    pub fn init() -> Result<Publisher, Box<std::error::Error>> {
        let matches = App::new(format!("{}", style("Fluence Code Publisher").blue().bold()))
            .version("0.1.0")
            .author(format!("{}", style("Fluence Labs").blue().bold()).as_str())
            .about(
                format!(
                    "{}",
                    style("Console utility for deploying code to fluence cluster")
                        .blue()
                        .bold()
                )
                .as_str(),
            )
            .arg(
                Arg::with_name("path")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("path to compiled `wasm` code"),
            )
            .arg(
                Arg::with_name("account")
                    .alias("account")
                    .required(true)
                    .alias("account")
                    .long("account")
                    .short("a")
                    .takes_value(true)
                    .help("ethereum account without `0x`"),
            )
            .arg(
                Arg::with_name("contract_address")
                    .alias("contract_address")
                    .required(true)
                    .takes_value(true)
                    .index(2)
                    .help("deployer contract address without `0x`"),
            )
            .arg(
                Arg::with_name("swarm_url")
                    .alias("swarm_url")
                    .long("swarm_url")
                    .short("s")
                    .required(false)
                    .takes_value(true)
                    .help("http address to swarm node")
                    .default_value("http://localhost:8500/"),
            ) //todo: use public gateway
            .arg(
                Arg::with_name("eth_url")
                    .alias("eth_url")
                    .long("eth_url")
                    .short("e")
                    .required(false)
                    .takes_value(true)
                    .help("http address to ethereum node")
                    .default_value("http://localhost:8545/"),
            ) //todo: use public node or add light client
            .arg(
                Arg::with_name("password")
                    .alias("password")
                    .long("password")
                    .short("p")
                    .required(false)
                    .takes_value(true)
                    .help("password to unlock account in ethereum client"),
            )
            .arg(
                Arg::with_name("cluster_size")
                    .alias("cluster_size")
                    .long("cluster_size")
                    .short("cs")
                    .required(false)
                    .takes_value(true)
                    .default_value("3")
                    .help("cluster's size that needed to deploy this code"),
            )
            .get_matches();

        let path = matches.value_of("path").unwrap().to_string();

        let contract_address = matches.value_of("contract_address").unwrap().to_string();
        let contract_address: Address = contract_address.parse()?;

        let account = matches.value_of("account").unwrap();
        let account: Address = account.parse()?;

        let swarm_url = matches.value_of("swarm_url").unwrap().to_string();
        let eth_url = matches.value_of("eth_url").unwrap().to_string();

        let password = matches.value_of("password").map(|s| s.to_string());

        let cluster_size: u64 = matches.value_of("cluster_size").unwrap().parse()?;
        if cluster_size < 1 || cluster_size > 255 {
            panic!("Invalid number: {}. Must be from 1 to 255.", cluster_size);
        }

        Ok(Publisher {
            path,
            contract_address,
            account,
            swarm_url,
            eth_url,
            password,
            cluster_size,
        })
    }
}
