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

use crate::config::config::{ClientType, JanusConfig};
use crate::node_service::NodeService;
use crate::peer_service::{libp2p, websocket};

use ::config as fileconfig;
use clap::{App, ArgMatches};
use ctrlc;
use env_logger;
use exitfailure::ExitFailure;
use futures::channel::{mpsc, oneshot};
use log::trace;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod config;
mod error;
mod misc;
mod node_service;
mod peer_service;
mod trust;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const CONFIG_FILE_NAME: &str = "./config";

fn main() -> Result<(), ExitFailure> {
    env_logger::init();

    let arg_matches = App::new("Fluence Janus protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(&config::args::prepare_args())
        .get_matches();

    let janus_config = generate_config(arg_matches, CONFIG_FILE_NAME)?;
    let janus = start_janus(janus_config)?;

    println!("Janus has been successfully started.\nWaiting for Ctrl-C for exit");
    wait_ctrc(std::time::Duration::from_secs(2));

    println!("shutdown services");
    janus.stop();

    Ok(())
}

trait Stoppable {
    fn stop(self);
}

// for stop Janus just call stop() of the result object
fn start_janus(config: JanusConfig) -> Result<impl Stoppable, std::io::Error> {
    trace!("starting Janus");

    // peer_outlet  – to send events from node service to peer service
    // peer_inlet   – to receive these events in peer service
    let (peer_outlet, peer_inlet) = mpsc::unbounded();

    // node_outlet  – to send events from peer service to node service
    // node_inlet   – to receive these events in node service
    let (node_outlet, node_inlet) = mpsc::unbounded();

    let peer_exit_outlet = match config.node_service_config.client {
        ClientType::Libp2p => {
            libp2p::start_peer_service(config.peer_service_config, peer_inlet, node_outlet)
        }
        ClientType::Websocket => {
            websocket::start_peer_service(config.websocket_config, peer_inlet, node_outlet)
        }
    };

    let node_service = NodeService::new(config.node_service_config);
    let node_exit_outlet = node_service.start(node_inlet, peer_outlet);

    struct Janus {
        node_exit_outlet: oneshot::Sender<()>,
        #[allow(dead_code)]
        peer_exit_outlet: oneshot::Sender<()>,
    }

    impl Stoppable for Janus {
        fn stop(self) {
            // shutting down node service leads to shutting down peer service by canceling the mpsc channel
            let _ = self.peer_exit_outlet.send(());
            self.node_exit_outlet.send(()).unwrap();
        }
    }

    Ok(Janus {
        node_exit_outlet,
        peer_exit_outlet,
    })
}

// waits for Ctrl+C pressing on a keyboard
// sleeps sleep_interval between checking of Ctrl+C being pressed
fn wait_ctrc(sleep_interval: std::time::Duration) {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(sleep_interval);
    }
}

// generates config from arguments and a config file
fn generate_config(
    arg_matches: ArgMatches,
    file_name: &str,
) -> Result<JanusConfig, failure::Error> {
    let mut settings = fileconfig::Config::default();
    settings
        .merge(fileconfig::File::with_name(file_name).required(false))
        .unwrap();
    let file_config = settings.try_into::<HashMap<String, String>>().unwrap();
    config::config::generate_config(arg_matches, file_config)
}
