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

use clap::App;
use futures::channel::oneshot;

use ctrlc_adapter::block_until_ctrlc;
use env_logger::Env;
use eyre::WrapErr;
use log::LevelFilter;
use particle_node::{
    config::{certificates, create_args},
    write_default_air_interpreter, Node,
};
use server_config::{load_config, ResolvedConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

trait Stoppable {
    fn stop(self);
}

fn main() -> eyre::Result<()> {
    // TODO: maybe set log level via flag?
    env_logger::from_env(Env::default().default_filter_or("INFO"))
        .format_timestamp_micros()
        // Disable most spamming modules
        .filter_module("cranelift_codegen", LevelFilter::Off)
        .filter_module("wasmer_wasi_fl", LevelFilter::Off)
        .init();

    let version = format!("{}; AIR version {}", VERSION, air_interpreter_wasm::VERSION);
    let arg_matches = App::new("Fluence node")
        .version(version.as_str())
        .author(AUTHORS)
        .about(DESCRIPTION)
        .args(create_args().as_slice())
        .get_matches();

    log::info!(
        r#"
+-------------------------------------------------+
| Hello from the Fluence Team. If you encounter   |
| any troubles with node operation, please update |
| the node via                                    |
|     docker pull fluencelabs/fluence:latest      |
|                                                 |
| or contact us at                                |
| github.com/fluencelabs/fluence/discussions      |
+-------------------------------------------------+
    "#
    );

    let config = load_config(arg_matches)?;

    write_default_air_interpreter(&config.dir_config.air_interpreter_path)?;
    log::info!(
        "AIR interpreter: {:?}",
        config.dir_config.air_interpreter_path
    );

    let fluence = start_fluence(config)?;
    log::info!("Fluence has been successfully started.");

    log::info!("Waiting for Ctrl-C to exit...");
    block_until_ctrlc();

    log::info!("Shutting down...");
    fluence.stop();

    Ok(())
}

// NOTE: to stop Fluence just call Stoppable::stop()
fn start_fluence(config: ResolvedConfig) -> eyre::Result<impl Stoppable> {
    log::trace!("starting Fluence");

    certificates::init(&config.dir_config.certificate_dir, &config.root_key_pair)
        .wrap_err("failed to init certificates")?;

    let key_pair = config.root_key_pair.clone();
    log::info!(
        "public key = {}",
        bs58::encode(key_pair.public().to_vec()).into_string()
    );

    let listen_config = config.listen_config();
    let mut node = Node::new(key_pair.into(), config).wrap_err("create node instance")?;
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
