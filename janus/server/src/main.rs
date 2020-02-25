/*
 * Copyright 2020 Fluence Labs Limited
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

#![recursion_limit = "512"]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

extern crate config as fileconfig;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use clap::App;
use ctrlc;
use env_logger;
use exitfailure::ExitFailure;
use futures::channel::{mpsc, oneshot};
use log::trace;

use crate::config::config::{ClientType, JanusConfig};
use crate::node_service::NodeService;
use crate::peer_service::{libp2p, websocket};

mod config;
mod error;
mod misc;
mod node_service;
mod peer_service;
mod trust;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn start_janus(
    config: JanusConfig,
) -> Result<(oneshot::Sender<()>, oneshot::Sender<()>), std::io::Error> {
    trace!("starting Janus");

    // out_sender – to send events from node service to peer service
    // out_receiver – to receive these events in peer service
    let (out_sender, out_receiver) = mpsc::unbounded();

    // in_sender – to send events from peer service to node service
    // in_receiver – to receive these events in node service
    let (in_sender, in_receiver) = mpsc::unbounded();

    let exit_sender = match config.node_service_config.client {
        ClientType::Libp2p => {
            libp2p::start_peer_service(config.peer_service_config, out_receiver, in_sender)
        }
        ClientType::Websocket => {
            websocket::start_peer_service(config.websocket_config, out_receiver, in_sender)
        }
    };

    let node_service = NodeService::new(config.node_service_config);
    let node_service_exit = node_service.start(in_receiver, out_sender);

    Ok((node_service_exit, exit_sender))
}

fn main() -> Result<(), ExitFailure> {
    env_logger::init();

    let arg_matches = App::new("Fluence Janus protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(&config::args::prepare_args())
        .get_matches();

    let mut settings = fileconfig::Config::default();
    settings
        .merge(fileconfig::File::with_name("Config").required(false))
        .unwrap();
    let file_config = settings.try_into::<HashMap<String, String>>().unwrap();

    let config = config::config::gen_config(arg_matches, file_config)?;

    println!("Janus is starting...");

    let (node_service_exit, _peer_service_exit) = start_janus(config)?;

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

    // shutting down node service leads to shutting down peer service by canceling the mpsc channel
    node_service_exit.send(()).unwrap();

    Ok(())
}
