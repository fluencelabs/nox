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

use crate::config::JanusConfig;
use crate::node_service::NodeService;

use clap::App;
use futures::channel::oneshot;
use log::trace;

use std::error::Error;

mod certificate_storage;
mod config;
mod error;
pub mod key_storage;
pub mod misc;
pub mod node_service;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let arg_matches = App::new("Fluence Janus protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(&config::prepare_args())
        .get_matches();

    let janus_config = config::load_config(arg_matches)?;
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
fn start_janus(config: JanusConfig) -> Result<impl Stoppable, Box<dyn std::error::Error>> {
    trace!("starting Janus");

    certificate_storage::init(config.certificate_dir.as_str(), &config.root_key_pair)?;

    let libp2p_kp = &config.root_key_pair.key_pair;
    println!(
        "public key = {}",
        bs58::encode(libp2p_kp.public().encode().to_vec().as_slice()).into_string()
    );

    let node_service = NodeService::new(
        libp2p_kp.clone(),
        config.node_service_config,
        config.root_weights,
    );

    let node_exit_outlet = node_service.start();

    struct Janus {
        node_exit_outlet: oneshot::Sender<()>,
    }

    impl Stoppable for Janus {
        fn stop(self) {
            // shutting down node service leads to shutting down peer service by canceling the mpsc channel
            self.node_exit_outlet.send(()).unwrap();
        }
    }

    Ok(Janus { node_exit_outlet })
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
