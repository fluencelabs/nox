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

pub mod app;
pub mod status;
pub mod ui;

use self::status::{get_status, Status};
use crate::command::*;
use crate::contract_status::app::{App, Node};
use crate::contract_status::ui::rich_status;
use crate::utils;
use clap::Arg;
use clap::{App as ClapApp, ArgMatches, SubCommand, _clap_count_exprs, arg_enum};
use console::style;
use failure::err_msg;
use failure::Error;
use failure::ResultExt;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::net::IpAddr;
use web3::types::Address;
use web3::types::H256;

const INTERACTIVE: &str = "interactive";
const OWNER: &str = "owner";
const APP_ID: &str = "app_id";
const FILTER_MODE: &str = "filter_mode";

// Implements logical 'and' for variadic number of Option<bool>
// Example of generated code:
//    opt_and!(Some(true), Some(false), None) will generate:
//
//    Some(true).unwrap_or(true) && Some(false).unwrap_or(true) && None.unwrap_or(true)
//
macro_rules! opt_and {
    ($head:ident, $($tail:ident),*) => {{
        {
            $head.unwrap_or(true) && opt_and! { $($tail),* }
        }
    }};

    ($last:ident) => {{
        $last.unwrap_or(true)
    }};
}

// Implements logical 'or' for variadic number of Option<bool>
// Example of generated code:
//    opt_and!(Some(false), None, Some(true)) will generate:
//
//    Some(false).unwrap_or(false) || None.unwrap_or(false) || Some(true).unwrap_or(false)
//
macro_rules! opt_or {
    ($head:ident, $($tail:ident),*) =>  {{
        {
            $head.unwrap_or(false) || opt_or! { $($tail),* }
        }
    }};

    ($last:ident) => {{
        $last.unwrap_or(false)
    }};
}

// arg_enum generates FromStr for FilterMode enum
arg_enum! {
    #[derive(Debug)]
    // Logical mode for filtering
    enum FilterMode {
        // Element is displayed if it matches all passed filters. Conjunction mode.
        And,
        // Element is displayed if it matches any of the passed filters. Disjunction mode.
        Or
    }
}

// Data to be used for filtering
struct StatusFilter {
    // logical mode
    mode: FilterMode,
    // filter node or app by owner
    owner: Option<Address>,
    // filter app by app_id, nodes by apps they're hosting
    app_id: Option<u64>,
    // filter nodes by IP address, doesn't filter apps
    node_ip: Option<IpAddr>,
    // filter nodes by tendermint validator key (used as node id), apps by nodes in cluster
    tendermint_key: Option<H256>,
}

impl StatusFilter {
    // Parse filters from command line options
    // TODO: remove 'map_err(err_msg).context(...)` boilerplate
    fn from_args(args: &ArgMatches) -> Result<StatusFilter, Error> {
        let mode: FilterMode = utils::get_opt(args, FILTER_MODE)
            .map_err(err_msg)
            .context("error parsing filter mode")?
            .unwrap_or(FilterMode::And);
        let owner: Option<Address> = utils::get_opt_hex(args, OWNER)
            .map_err(err_msg)
            .context("error parsing owner")?;
        let app_id: Option<u64> = utils::get_opt(args, APP_ID)
            .map_err(err_msg)
            .context("error parsing app_id")?;
        let node_ip: Option<IpAddr> = utils::get_opt(args, NODE_IP)
            .map_err(err_msg)
            .context("error parsing node_ip")?;
        let tendermint_key: Option<H256> = if args.is_present(TENDERMINT_KEY) {
            Some(parse_tendermint_key(args)?)
        } else {
            None
        };

        Ok(StatusFilter {
            mode,
            owner,
            app_id,
            node_ip,
            tendermint_key,
        })
    }

    // filters existing status to a new one by filtering and cloning all elements
    fn filter(&self, status: &Status) -> Status {
        let nodes: Vec<Node> = status
            .nodes
            .iter()
            .filter(|node| {
                let by_app_id = self.app_id.map(|f| {
                    node.app_ids
                        .as_ref()
                        .map(|ids| ids.contains(&f))
                        .unwrap_or(false)
                });
                let by_owner = self.owner.map(|f| f == node.owner);
                let by_node_ip = self.node_ip.map(|f| f == node.ip_addr);
                let by_t_key = self.tendermint_key.map(|f| f == node.validator_key);
                match self.mode {
                    FilterMode::And => opt_and!(by_owner, by_app_id, by_node_ip, by_t_key),
                    FilterMode::Or => opt_or!(by_owner, by_app_id, by_node_ip, by_t_key),
                }
            })
            .cloned()
            .collect();

        // used to also display apps for these nodes
        let node_ids: HashSet<H256> = nodes.iter().map(|n| n.validator_key).collect();

        let apps: Vec<App> = status
            .apps
            .iter()
            .filter(|app| {
                let by_app_id = self.app_id.map(|f| f == app.app_id);
                let by_owner = self.owner.map(|f| f == app.owner);
                let by_t_key = self.tendermint_key.and_then(|f| {
                    app.cluster
                        .as_ref()
                        .map(|c| c.node_ids.contains(&f) || node_ids.contains(&f))
                });
                match self.mode {
                    FilterMode::And => opt_and!(by_owner, by_app_id, by_t_key),
                    FilterMode::Or => opt_or!(by_owner, by_app_id, by_t_key),
                }
            })
            .cloned()
            .collect();

        Status { nodes, apps }
    }
}

lazy_static! {
    static ref AFTER_HELP: String = format!(
        r#"
FILTERING EXAMPLES:
    Show nodes that have specified id or specified IP address:
{}

    Show nodes that both have specified owner and specified IP address:
{}

NOTE:
    Apps hosted by any of the displayed nodes will also be displayed
            "#,
        style(
            r#"       ./fluence status \
            --contract_address <contract address> \
            --tendermint_key <tendermint key> \
            --node_ip <ip address> \
            --filter_mode or"#
        )
        .cyan(),
        style(
            r#"       ./fluence status \
            --contract_address <contract address> \
            --node_ip <ip address> \
            --owner <ethereum address> \
            --filter_mode and"#
        )
        .cyan()
    );
}

pub fn subcommand<'a, 'b>() -> ClapApp<'a, 'b> {
    SubCommand::with_name("status")
        .about("Show state of the Fluence network as seen by the smart-contract")
        .after_help(AFTER_HELP.as_str())
        .args(&[
            contract_address().display_order(0),
            eth_url().display_order(1),
            Arg::with_name(INTERACTIVE)
                .long(INTERACTIVE)
                .short("I")
                .required(false)
                .takes_value(false)
                .help("Show status as an interactive table")
                .display_order(2),
            Arg::with_name(OWNER)
                .long(OWNER)
                .short(OWNER)
                .required(false)
                .takes_value(true)
                .value_name("eth address")
                .help("Filter nodes and apps owned by this Ethereum address")
                .display_order(3),
            Arg::with_name(APP_ID)
                .long(APP_ID)
                .short(APP_ID)
                .required(false)
                .takes_value(true)
                .help("Filter nodes and apps by app id")
                .display_order(3),
            node_ip()
                .required(false)
                .help("Filter nodes by IP address")
                .display_order(3),
            tendermint_key()
                .required(false)
                .help("Filter nodes and apps by Tendermint validator key (node id)")
                .display_order(3),
            Arg::with_name(FILTER_MODE)
                .long(FILTER_MODE)
                .short(FILTER_MODE)
                .required(false)
                .takes_value(true)
                .value_name("and|or")
                .default_value("and")
                .help("Logical mode of the filter")
                .display_order(4),
        ])
}

/// Gets status about Fluence contract from ethereum blockchain.
pub fn get_status_by_args(args: &ArgMatches) -> Result<Option<Status>, Error> {
    let eth_url = parse_eth_url(args)?;
    let contract_address: Address = parse_contract_address(args)?;

    let filter = StatusFilter::from_args(args)?;

    let status = get_status(eth_url.as_str(), contract_address)?;

    let status = filter.filter(&status);

    if args.is_present(INTERACTIVE) {
        rich_status::draw(&status)?;
        Ok(None)
    } else {
        Ok(Some(status))
    }
}

#[cfg(test)]
mod tests {
    use super::get_status;
    use crate::command::EthereumArgs;
    use crate::publisher::Publisher;
    use crate::register::Register;
    use failure::Error;
    use rand::prelude::*;
    use web3::types::*;

    const CONTRACT_ADDR: &str = "9995882876ae612bfd829498ccd73dd962ec950a";
    const SWARM_URL: &str = "http://localhost:8500";

    fn generate_publisher(bytes: Vec<u8>, cluster_size: &u8) -> Publisher {
        let eth = EthereumArgs::default();

        Publisher::new(
            bytes,
            SWARM_URL.to_string(),
            cluster_size.to_owned(),
            vec![],
            eth,
        )
    }

    fn generate_register(address: &str, start_port: u16, last_port: u16) -> Register {
        let mut rng = rand::thread_rng();
        let rnd_num: u64 = rng.gen();

        let tendermint_key: H256 = H256::from(rnd_num);
        let tendermint_node_id: H160 = H160::from(rnd_num);

        let eth = EthereumArgs::default();

        Register::new(
            address.parse().unwrap(),
            tendermint_key,
            tendermint_node_id,
            start_port,
            last_port,
            false,
            eth,
        )
        .unwrap()
    }

    #[test]
    fn status_correct() -> Result<(), Error> {
        let reg1_ip = "156.56.34.67";
        let reg1_start_port = 2004;
        let reg1_end_port = 2016;
        let register1 = generate_register(reg1_ip, reg1_start_port, reg1_end_port);

        let reg2_ip = "92.156.134.98";
        let reg2_start_port = 5207;
        let reg2_end_port = 6118;
        let register2 = generate_register(reg2_ip, reg2_start_port, reg2_end_port);

        register1.register(false)?;
        register2.register(false)?;

        let bytes1 = vec![1, 2, 3];
        let cluster_size1 = &2;
        let publisher1 = generate_publisher(bytes1, cluster_size1);

        let bytes2 = vec![1, 2, 3, 4, 5];
        let cluster_size2 = &180;
        let publisher2 = generate_publisher(bytes2, cluster_size2);

        publisher1.publish(false)?;
        publisher2.publish(false)?;

        let _status = get_status(
            register1.eth().eth_url.as_str(),
            CONTRACT_ADDR.parse().unwrap(),
        )?;

        //let clusters = status.clusters();
        //let apps = status.enqueued_apps();
        //let nodes = status.ready_nodes();

        //        let cluster = clusters
        //            .iter()
        //            .find(|&cl| cl.code().cluster_size() == cluster_size1);
        //
        //        println!("{:?}", clusters);
        //
        //        assert_eq!(cluster.is_some(), true);
        //
        //        let node1 = nodes.iter().find(|&n| n.ip_addr() == reg1_ip);
        //
        //        assert_eq!(node1.is_some(), true);
        //        let node1 = node1.unwrap();
        //        assert_eq!(node1.start_port(), &reg1_start_port);
        //        assert_eq!(node1.end_port(), &reg1_end_port);
        //
        //        let node2 = nodes.iter().find(|&n| n.ip_addr() == reg2_ip);
        //
        //        assert_eq!(node2.is_some(), true);
        //        let node2 = node2.unwrap();
        //        assert_eq!(node2.start_port(), &reg2_start_port);
        //        assert_eq!(node2.end_port(), &reg2_end_port);

        //        let app = apps.iter().find(|&c| c.cluster_size() == cluster_size2);
        //        assert_eq!(app.is_some(), true);

        Ok(())
    }
}
