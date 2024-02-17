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
use eyre::WrapErr;
use libp2p::PeerId;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::oneshot;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use air_interpreter_fs::write_default_air_interpreter;
use aquamarine::{DataStoreConfig, VmConfig};
use avm_server::avm_runner::AVMRunner;
use config_utils::to_peer_id;
use core_manager::manager::{CoreManager, CoreManagerFunctions, PersistentCoreManager};
use fs_utils::to_abs_path;
use nox::{env_filter, log_layer, tokio_console_layer, tracing_layer, Node};
use server_config::{load_config, ConfigData, ResolvedConfig};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

trait Stoppable {
    fn stop(self);
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> eyre::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let version = format!("{}; AIR version {}", VERSION, air_interpreter_wasm::VERSION);
    let authors = format!("by {AUTHORS}");
    let config_data = ConfigData {
        binary_name: PKG_NAME.to_string(),
        version,
        authors,
        description: DESCRIPTION.to_string(),
    };
    let config = load_config(Some(config_data))?;

    match config.no_banner {
        Some(true) => {}
        _ => {
            println!(
                r#"
+-------------------------------------------------+
| Hello from the Fluence Team. If you encounter   |
| any troubles with node operation, please update |
| the node via                                    |
|     docker pull fluencelabs/nox:latest          |
|                                                 |
| or contact us at                                |
| https://discord.com/invite/5qSnPZKh7u           |
+-------------------------------------------------+
    "#
            )
        }
    }

    let resolved_config = config.clone().resolve()?;

    let (core_manager, core_manager_task) = PersistentCoreManager::from_path(
        resolved_config.dir_config.core_state_path.clone(),
        resolved_config.node_config.system_cpu_count,
        resolved_config.node_config.cpus_range.clone(),
    )?;

    let core_manager: Arc<CoreManager> = Arc::new(core_manager.into());

    let system_cpu_cores_assignment = core_manager.system_cpu_assignment();

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.worker_threads(system_cpu_cores_assignment.logical_core_ids.len());
    builder.on_thread_start(move || system_cpu_cores_assignment.pin_current_thread());

    builder.enable_all();

    let enable_histogram = config.node_config.metrics_config.tokio_metrics_enabled
        && config
            .node_config
            .metrics_config
            .tokio_metrics_poll_histogram_enabled;
    if enable_histogram {
        builder.enable_metrics_poll_count_histogram();
    }

    builder
        .build()
        .expect("Could not make tokio runtime")
        .block_on(async {
            core_manager_task.run(core_manager.clone()).await;

            let key_pair = resolved_config.node_config.root_key_pair.clone();
            let base64_key_pair = base64.encode(key_pair.public().to_vec());
            let peer_id = to_peer_id(&key_pair.into());

            tracing_subscriber::registry()
                .with(env_filter())
                .with(log_layer(&config.log))
                .with(tokio_console_layer(&config.console)?)
                .with(tracing_layer(&config.tracing, peer_id, VERSION)?)
                .init();

            if let Some(true) = config.print_config {
                log::info!("Loaded config: {:#?}", config);
            }

            log::info!("node public key = {}", base64_key_pair);
            log::info!("node server peer id = {}", peer_id);

            let interpreter_path =
                to_abs_path(resolved_config.dir_config.air_interpreter_path.clone());
            write_default_air_interpreter(&interpreter_path)?;
            log::info!("AIR interpreter: {:?}", interpreter_path);

            let fluence = start_fluence(resolved_config, core_manager, peer_id).await?;
            log::info!("Fluence has been successfully started.");
            log::info!("Waiting for Ctrl-C to exit...");

            signal::ctrl_c().await.expect("Failed to listen for event");
            log::info!("Shutting down...");

            fluence.stop();
            Ok(())
        })
}

// NOTE: to stop Fluence just call Stoppable::stop()
async fn start_fluence(
    config: ResolvedConfig,
    core_manager: Arc<CoreManager>,
    peer_id: PeerId,
) -> eyre::Result<impl Stoppable> {
    log::trace!("starting Fluence");

    let listen_addrs = config.listen_multiaddrs();
    let vm_config = vm_config(&config);
    let data_store_config = DataStoreConfig::new(config.dir_config.avm_base_dir.clone());

    let system_services_config = config.system_services.clone();
    let system_service_distros =
        system_services::SystemServiceDistros::default_from(system_services_config)
            .wrap_err("Failed to get default system service distros")?;

    let mut node: Box<Node<AVMRunner>> = Node::new(
        config,
        core_manager,
        vm_config,
        data_store_config,
        VERSION,
        air_interpreter_wasm::VERSION,
        system_service_distros,
    )
    .await
    .wrap_err("error create node instance")?;
    node.listen(listen_addrs).wrap_err("error on listen")?;

    let started_node = node.start(peer_id).await.wrap_err("node failed to start")?;

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

    Ok(Fluence {
        node_exit_outlet: started_node.exit_outlet,
    })
}

fn vm_config(config: &ResolvedConfig) -> VmConfig {
    VmConfig::new(
        to_peer_id(&config.root_key_pair.clone().into()),
        config.dir_config.air_interpreter_path.clone(),
        config
            .node_config
            .aquavm_heap_size_limit
            .map(|byte_size| byte_size.as_u64()),
    )
}
