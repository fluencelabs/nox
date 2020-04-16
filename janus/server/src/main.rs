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
use ctrlc_adapter::block_until_ctrlc;
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
        .args(config::prepare_args().as_slice())
        .get_matches();

    let janus_config = config::load_config(arg_matches)?;

    let janus = start_janus(janus_config)?;
    println!("Janus has been successfully started.");

    println!("Waiting for Ctrl-C to exit");
    block_until_ctrlc();

    println!("shutdown services");
    janus.stop();

    Ok(())
}

trait Stoppable {
    fn stop(self);
}

// for stop Janus just call stop() of the result object
fn start_janus(config: JanusConfig) -> Result<impl Stoppable, Box<dyn Error>> {
    trace!("starting Janus");

    certificate_storage::init(config.certificate_dir.as_str(), &config.root_key_pair)?;

    let key_pair = &config.root_key_pair.key_pair;
    println!(
        "public key = {}",
        bs58::encode(key_pair.public().encode().to_vec().as_slice()).into_string()
    );

    let node_service = NodeService::new(
        key_pair.clone(),
        config.node_service_config.clone(),
        config.root_weights.clone(),
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
