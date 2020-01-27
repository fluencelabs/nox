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

#![feature(impl_trait_in_bindings)]

mod config;
mod error;
mod node_service;
mod peer_service;

use crate::config::{NodeServiceConfig, PeerServiceConfig, WebsocketConfig};
use crate::node_service::node_service::{start_node_service, NodeService};
use crate::peer_service::notifications::{InPeerNotification, OutPeerNotification};
use crate::peer_service::peer_service::{start_peer_service, PeerService};
use clap::{App, Arg, ArgMatches};
use ctrlc;
use async_std::task;
use futures::channel::{mpsc, oneshot};
use futures::{future::select, StreamExt};
use env_logger;
use exitfailure::ExitFailure;
use failure::_core::str::FromStr;
use log::trace;
use parity_multiaddr::Multiaddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tungstenite::WebSocket;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

const PEER_SERVICE_PORT: &str = "peer-service-port";
const NODE_SERVICE_PORT: &str = "node-service-port";
const WEBSOCKET_PORT: &str = "websocket-port";
const BOOTSTRAP_NODE: &str = "bootstrap-node";

fn prepare_args<'a, 'b>() -> [Arg<'a, 'b>; 4] {
    [
        Arg::with_name(PEER_SERVICE_PORT)
            .takes_value(true)
            .short("pp")
            .default_value("9999")
            .help("port that will be used by the peer service"),
        Arg::with_name(WEBSOCKET_PORT)
            .takes_value(true)
            .short("w")
            .default_value("8888")
            .help("port that will be used by the websocket service"),
        Arg::with_name(NODE_SERVICE_PORT)
            .takes_value(true)
            .short("np")
            .default_value("7777")
            .help("port that will be used by the node service"),
        Arg::with_name(BOOTSTRAP_NODE)
            .takes_value(true)
            .short("b")
            .multiple(true)
            .help("bootstrap nodes of the Fluence network"),
    ]
}

fn make_configs_from_args(
    arg_matches: ArgMatches,
) -> Result<(NodeServiceConfig, PeerServiceConfig, WebsocketConfig), ExitFailure> {
    let mut node_service_config = NodeServiceConfig::default();
    let mut peer_service_config = PeerServiceConfig::default();
    let mut websocket_config = WebsocketConfig::default();

    if let Some(peer_port) = arg_matches.value_of(PEER_SERVICE_PORT) {
        let peer_port: u16 = u16::from_str(peer_port)?;
        peer_service_config.listen_port = peer_port;
    }

    if let Some(node_port) = arg_matches.value_of(NODE_SERVICE_PORT) {
        let node_port: u16 = u16::from_str(node_port)?;
        node_service_config.listen_port = node_port;
    }

    if let Some(websocket_port) = arg_matches.value_of(WEBSOCKET_PORT) {
        let websocket_port: u16 = u16::from_str(websocket_port)?;
        websocket_config.listen_port = websocket_port;
    }

    if let Some(bootstrap_node) = arg_matches.value_of(BOOTSTRAP_NODE) {
        let bootstrap_node = Multiaddr::from_str(bootstrap_node)?;
        node_service_config.bootstrap_nodes.push(bootstrap_node);
    }

    Ok((node_service_config, peer_service_config, websocket_config))
}

async fn start_janus(
    node_service_config: NodeServiceConfig,
    peer_service_config: PeerServiceConfig,
    websocket_config: WebsocketConfig
) -> Result<(oneshot::Sender<()>, oneshot::Sender<()>), std::io::Error> {
    trace!("starting Janus");

    let (channel_in_1, channel_out_1) = mpsc::unbounded();
    let (channel_in_2, channel_out_2) = mpsc::unbounded();

    let (exit_sender, exit_receiver) = oneshot::channel();

    let node_service = NodeService::new(node_service_config);
    let node_service_exit = start_node_service(
        node_service,
        channel_out_2,
        channel_in_1,
    );

    let f = node_service::websocket::websocket::start_peer_service(websocket_config, channel_out_1, channel_in_2).await;

    Ok((node_service_exit, exit_sender))
}

fn main() -> Result<(), ExitFailure> {
    env_logger::init();

    let arg_matches = App::new("Fluence Janus protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(&prepare_args())
        .get_matches();

    println!("Janus is starting...");

    let (node_service_config, peer_service_config, websocket_config) = make_configs_from_args(arg_matches)?;
    let (node_service_exit, peer_service_exit) =
        task::block_on(start_janus(node_service_config, peer_service_config, websocket_config))?;

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

    node_service_exit.send(()).unwrap();
    peer_service_exit.send(()).unwrap();

    Ok(())
}
