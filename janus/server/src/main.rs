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

use crate::config::config::JanusConfig;
use crate::node_service::NodeService;
use crate::peer_service::PeerService;

use ::config as fileconfig;
use clap::{App, ArgMatches};
use ctrlc;
use env_logger;
use exitfailure::ExitFailure;
use futures::channel::oneshot;
use log::trace;

use std::collections::HashMap;

use libp2p::identity::Keypair;

mod certificate_storage;
mod config;
mod error;
mod key_storage;
pub mod misc;
mod node_service;
mod peer_service;

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
    block_until_ctrlc();

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

    let root_key_pair = key_storage::load_or_create_key_pair(config.secret_key_path.as_str())?;

    certificate_storage::init(config.certificate_dir.as_str(), &root_key_pair)?;

    let libp2p_kp = Keypair::Ed25519(root_key_pair.key_pair);
    let (peer_service, peer_outlet) =
        PeerService::new(libp2p_kp.clone(), config.peer_service_config);
    let (node_service, node_outlet) = NodeService::new(libp2p_kp, config.node_service_config);

    let peer_exit_outlet = peer_service.start(node_outlet);
    let node_exit_outlet = node_service.start(peer_outlet);

    struct Janus {
        node_exit_outlet: oneshot::Sender<()>,
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

// blocks until either SIGINT(Ctrl+C) or SIGTERM signals received
fn block_until_ctrlc() {
    let (ctrlc_outlet, ctrlc_inlet) = oneshot::channel();
    let ctrlc_outlet = std::cell::RefCell::new(Some(ctrlc_outlet));

    ctrlc::set_handler(move || {
        ctrlc_outlet
            .borrow_mut()
            .take()
            .expect("ctrlc_outlet must be set")
            .send(())
            .expect("sending shutdown signal failed");
    })
    .expect("Error while setting ctrlc handler");

    async_std::task::block_on(ctrlc_inlet).expect("exit oneshot failed");
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
