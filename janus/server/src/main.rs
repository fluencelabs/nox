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

#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod config;
mod error;
mod node_service;
mod peer_service;

use crate::config::{NodeServiceConfig, PeerServiceConfig};
use crate::node_service::node_service::{start_node_service, NodeService, NodeServiceDescriptor};
use crate::peer_service::peer_service::{start_peer_service, PeerService};
use clap::{App, Arg, ArgMatches};
use ctrlc;
use env_logger;
use exitfailure::ExitFailure;
use failure::_core::str::FromStr;
use log::trace;
use parity_multiaddr::Multiaddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::runtime::Runtime;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const PEER_SERVICE_PORT: &str = "peer-service-port";
const NODE_SERVICE_PORT: &str = "node-service-port";
const BOOTSTRAP_NODE: &str = "bootstrap-node";

fn prepare_args<'a, 'b>() -> [Arg<'a, 'b>; 3] {
    [
        Arg::with_name(PEER_SERVICE_PORT)
            .takes_value(true)
            .short("pp")
            .default_value("7777")
            .help("port that will be used by the peer service"),
        Arg::with_name(NODE_SERVICE_PORT)
            .takes_value(true)
            .short("np")
            .default_value("9999")
            .help("port that will be used by the node service"),
        Arg::with_name(BOOTSTRAP_NODE)
            .takes_value(true)
            .short("b")
            .help("bootstrap nodes of the Fluence network"),
    ]
}

fn make_configs_from_args(
    arg_matches: ArgMatches,
) -> Result<(PeerServiceConfig, NodeServiceConfig), ExitFailure> {
    let mut peer_service_config = PeerServiceConfig::default();
    let mut node_service_config = NodeServiceConfig::default();

    if let Some(peer_port) = arg_matches.value_of(PEER_SERVICE_PORT) {
        let peer_port: u16 = u16::from_str(peer_port)?;
        peer_service_config.listen_port = peer_port;
    }

    if let Some(node_port) = arg_matches.value_of(NODE_SERVICE_PORT) {
        let node_port: u16 = u16::from_str(node_port)?;
        node_service_config.listen_port = node_port;
    }

    if let Some(bootstrap_node) = arg_matches.value_of(BOOTSTRAP_NODE) {
        let bootstrap_node = Multiaddr::from_str(bootstrap_node)?;
        peer_service_config.bootstrap_nodes.push(bootstrap_node);
    }

    Ok((peer_service_config, node_service_config))
}

fn start_janus(
    runtime: &Runtime,
    peer_service_config: PeerServiceConfig,
    node_service_config: NodeServiceConfig,
) -> Result<
    (
        tokio::sync::oneshot::Sender<()>,
        tokio::sync::oneshot::Sender<()>,
    ),
    std::io::Error,
> {
    trace!("starting Janus");

    let node_service = NodeService::new(node_service_config);
    let node_service_descriptor: NodeServiceDescriptor =
        start_node_service(node_service, &runtime.executor())
            .expect("An error occurred during node service start");

    let peer_service = PeerService::new(peer_service_config);
    let peer_service_exit = start_peer_service(
        peer_service,
        node_service_descriptor.node_channel_out,
        node_service_descriptor.node_channel_in,
        &runtime.executor(),
    )
    .expect("An error occurred during the peer service start");

    Ok((peer_service_exit, node_service_descriptor.exit_sender))
}

fn main() -> Result<(), ExitFailure> {
    env_logger::init();

    let runtime = tokio::runtime::Runtime::new()?;

    let arg_matches = App::new("Fluence Janus protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(&prepare_args())
        .get_matches();

    let (peer_service_config, node_service_config) = make_configs_from_args(arg_matches)?;
    let (peer_service_exit, node_service_exit) =
        start_janus(&runtime, peer_service_config, node_service_config)?;

    println!("Janus has been successfully started");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    while running.load(Ordering::SeqCst) {}

    println!("shutdown services");
    node_service_exit
        .send(())
        .expect("failed node service exiting");
    peer_service_exit
        .send(())
        .expect("failed peer service exiting");

    Ok(())
}
