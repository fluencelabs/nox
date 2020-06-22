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
#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

use fluence_server::config::{certificates, create_args, load_config, FluenceConfig, ServerConfig};
use fluence_server::Server;

use fluence_faas::{FluenceFaaS, IValue, RawCoreModulesConfig};

use clap::App;
use ctrlc_adapter::block_until_ctrlc;
use futures::channel::oneshot;
use std::collections::HashMap;
use std::error::Error;
use trust_graph::{KeyPair, PublicKeyHashable};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::builder()
        .format_timestamp_micros()
        .filter_level(log::LevelFilter::Info)
        .init();

    let arg_matches = App::new("Fluence protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(create_args().as_slice())
        .get_matches();

    let fluence_config = load_config(arg_matches)?;
    println!("config: {:?}", fluence_config);

    let fluence = start_fluence(fluence_config)?;
    log::info!("Fluence has been successfully started.");

    log::info!("Waiting for Ctrl-C to exit...");
    block_until_ctrlc();

    log::info!("Shutting down...");
    fluence.stop();

    Ok(())
}

trait Stoppable {
    fn stop(self);
}

// NOTE: to stop Fluence just call Stoppable::stop()
fn start_fluence(config: FluenceConfig) -> Result<impl Stoppable, Box<dyn Error>> {
    log::trace!("starting Fluence");

    certificates::init(config.certificate_dir.as_str(), &config.root_key_pair)?;

    let key_pair = &config.root_key_pair.key_pair;
    log::info!(
        "public key = {}",
        bs58::encode(key_pair.public().encode().to_vec().as_slice()).into_string()
    );

    let faas = FluenceFaaS::with_raw_config(config.faas);

    let node_service = Server::new(
        key_pair.clone(),
        config.server.clone(),
        config
            .root_weights
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect(),
    );

    let node_exit_outlet = node_service.start();

    struct Fluence {
        node_exit_outlet: oneshot::Sender<()>,
    }

    impl Stoppable for Fluence {
        fn stop(self) {
            self.node_exit_outlet.send(()).unwrap();
        }
    }

    Ok(Fluence { node_exit_outlet })
}
