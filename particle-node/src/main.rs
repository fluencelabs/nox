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

use particle_node::{
    config::{certificates, create_args},
    write_default_air_interpreter, Node,
};
use server_config::{default_air_interpreter_path, load_config, FluenceConfig};

use anyhow::Context;
use clap::App;
use ctrlc_adapter::block_until_ctrlc;
use futures::channel::oneshot;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

trait Stoppable {
    fn stop(self);
}

fn main() -> anyhow::Result<()> {
    // TODO: set level to info by default (todo: check that RUST_LOG will still work)
    // TODO: maybe set log level via flag?
    env_logger::builder().format_timestamp_micros().init();

    let arg_matches = App::new("Fluence protocol server")
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(create_args().as_slice())
        .get_matches();

    write_default_air_interpreter()?;

    let fluence_config = load_config(arg_matches)?;

    log::info!(
        "AIR interpreter: {:?}",
        fluence_config.server.air_interpreter_path
    );

    let fluence = start_fluence(fluence_config)?;
    log::info!("Fluence has been successfully started.");

    log::info!("Waiting for Ctrl-C to exit...");
    block_until_ctrlc();

    log::info!("Shutting down...");
    fluence.stop();

    Ok(())
}

// NOTE: to stop Fluence just call Stoppable::stop()
fn start_fluence(config: FluenceConfig) -> anyhow::Result<impl Stoppable> {
    log::trace!("starting Fluence");

    certificates::init(config.certificate_dir.as_str(), &config.root_key_pair)
        .context("failed to init certificates")?;

    let key_pair = &config.root_key_pair.key_pair;
    log::info!(
        "public key = {}",
        bs58::encode(key_pair.public().encode().to_vec().as_slice()).into_string()
    );

    let listen_config = config.server.listen_config();
    let mut node = Node::new(key_pair.clone(), config.server).context("failed to create server")?;
    node.listen(&listen_config)
        .expect("Error starting node listener");

    let node_exit_outlet = node.start();

    struct Fluence {
        node_exit_outlet: oneshot::Sender<()>,
    }

    impl Stoppable for Fluence {
        fn stop(self) {
            self.node_exit_outlet
                .send(())
                .expect("failed to stop node through exit outlet");
        }
    }

    Ok(Fluence { node_exit_outlet })
}
