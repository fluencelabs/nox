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

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use env_logger::Env;
use eyre::WrapErr;
use log::LevelFilter;
use tokio::signal;
use tokio::sync::oneshot;

use air_interpreter_fs::write_default_air_interpreter;
use aquamarine::{VmConfig, AVM};
use config_utils::to_peer_id;
use fs_utils::to_abs_path;
use particle_node::Node;
use server_config::{load_config, ConfigData, ResolvedConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

trait Stoppable {
    fn stop(self);
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    if std::env::var("TOKIO_CONSOLE_ENABLED").is_ok() {
        console_subscriber::init();
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("INFO"))
        .format_timestamp_micros()
        // Disable most spamming modules
        .filter_module("cranelift_codegen", LevelFilter::Off)
        .filter_module("walrus", LevelFilter::Off)
        .filter_module("polling", LevelFilter::Off)
        .filter_module("wasmer_wasi_fl", LevelFilter::Error)
        .filter_module("wasmer_interface_types_fl", LevelFilter::Error)
        .filter_module("wasmer_wasi", LevelFilter::Error)
        .filter_module("tide", LevelFilter::Error)
        .filter_module("tokio_threadpool", LevelFilter::Error)
        .filter_module("tokio_reactor", LevelFilter::Error)
        .filter_module("mio", LevelFilter::Error)
        .filter_module("tokio_io", LevelFilter::Error)
        .filter_module("soketto", LevelFilter::Error)
        .filter_module("cranelift_codegen", LevelFilter::Error)
        .filter_module("async_io", LevelFilter::Error)
        .filter_module("tracing", LevelFilter::Error)
        .filter_module("avm_server::runner", LevelFilter::Error)
        .init();

    let _version = format!("{}; AIR version {}", VERSION, air_interpreter_wasm::VERSION);
    let _authors = format!("by {AUTHORS}");
    /*    let arg_matches = App::new("Fluence node")
    .version(version.as_str())
    .author(authors.as_str())
    .about(DESCRIPTION)
    .override_usage(r#"particle-node [FLAGS] [OPTIONS]"#)
    .args(create_args().as_slice())
    .get_matches();*/

    log::info!(
        r#"
+-------------------------------------------------+
| Hello from the Fluence Team. If you encounter   |
| any troubles with node operation, please update |
| the node via                                    |
|     docker pull fluencelabs/rust-peer:latest    |
|                                                 |
| or contact us at                                |
| github.com/fluencelabs/fluence/discussions      |
+-------------------------------------------------+
    "#
    );
    let version = format!("{}; AIR version {}", VERSION, air_interpreter_wasm::VERSION);
    let authors = format!("by {AUTHORS}");
    let config_data = ConfigData {
        version,
        authors,
        description: DESCRIPTION.to_string(),
    };
    let config = load_config(Some(config_data))?;

    let interpreter_path = to_abs_path(config.dir_config.air_interpreter_path.clone());
    write_default_air_interpreter(&interpreter_path)?;
    log::info!("AIR interpreter: {:?}", interpreter_path);

    let fluence = start_fluence(config).await?;
    log::info!("Fluence has been successfully started.");

    log::info!("Waiting for Ctrl-C to exit...");

    signal::ctrl_c().await.expect("Failed to listen for event");

    log::info!("Shutting down...");
    fluence.stop();

    Ok(())
}

// NOTE: to stop Fluence just call Stoppable::stop()
async fn start_fluence(config: ResolvedConfig) -> eyre::Result<impl Stoppable> {
    log::trace!("starting Fluence");

    let key_pair = config.root_key_pair.clone();
    let base64_key_pair = base64.encode(key_pair.public().to_vec());
    let peer_id = to_peer_id(&key_pair.into());
    log::info!("node public key = {}", base64_key_pair);
    log::info!("node server peer id = {}", peer_id);

    let listen_addrs = config.listen_multiaddrs();
    let vm_config = vm_config(&config);

    let mut node: Box<Node<AVM<_>>> =
        Node::new(config, vm_config, VERSION).wrap_err("error create node instance")?;
    node.listen(listen_addrs).wrap_err("error on listen")?;

    let node_exit_outlet = node
        .start(Some(peer_id.to_string()))
        .await
        .wrap_err("node failed to start")?;

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

fn vm_config(config: &ResolvedConfig) -> VmConfig {
    VmConfig::new(
        to_peer_id(&config.root_key_pair.clone().into()),
        config.dir_config.avm_base_dir.clone(),
        config.dir_config.air_interpreter_path.clone(),
        config
            .node_config
            .aquavm_max_heap_size
            .map(|byte_size| byte_size.as_u64()),
    )
}
