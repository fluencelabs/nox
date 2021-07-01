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
    Node,
};

use air_interpreter_fs::write_default_air_interpreter;
use builtins_deployer::BuiltinsDeployer;
use config_utils::to_peer_id;
use ctrlc_adapter::block_until_ctrlc;
use server_config::{load_config, ResolvedConfig};

use clap::App;
use env_logger::Env;
use eyre::WrapErr;
use fs_utils::to_abs_path;
use futures::channel::oneshot;
use log::LevelFilter;

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
    let authors = format!("by {}", AUTHORS);
    let arg_matches = App::new("Fluence node")
        .version(version.as_str())
        .author(authors.as_str())
        .about(DESCRIPTION)
        .override_usage(r#"particle-node [FLAGS] [OPTIONS]"#)
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

    let interpreter_path = to_abs_path(config.dir_config.air_interpreter_path.clone());
    write_default_air_interpreter(&interpreter_path)?;
    log::info!("AIR interpreter: {:?}", interpreter_path);

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

    let builtins_dir = config.dir_config.builtins_base_dir.clone();
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

    let stepper = node.stepper_pool_api.clone();
    let startup_peer_id = to_peer_id(&node.startup_keypair);
    let local_peer_id = node.local_peer_id;
    let node_exit_outlet = node.start();

    let mut builtin_deployer =
        BuiltinsDeployer::new(startup_peer_id, local_peer_id, stepper, builtins_dir);

    builtin_deployer
        .deploy_builtin_services()
        .wrap_err("builtins deploy failed")?;

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
