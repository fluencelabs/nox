/*
 * Copyright 2024 Fluence DAO
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

use axum::async_trait;
use base64::{engine::general_purpose::STANDARD as base64, Engine};
use cpu_utils::pinning::ThreadPinner;
use cpu_utils::HwlocCPUTopology;
use eyre::WrapErr;
use libp2p::PeerId;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use air_interpreter_fs::write_default_air_interpreter;
use aquamarine::{AVMRunner, DataStoreConfig, VmConfig};
use config_utils::to_peer_id;
use core_distributor::{AcquireStrategy, CoreDistributor, PersistentCoreDistributor};
use fs_utils::to_abs_path;
use nox::{env_filter, log_layer, tracing_layer, Node};
use server_config::{load_config, ConfigData, ResolvedConfig};
use tracing_panic::panic_hook;
use tracing_subscriber::reload;
use tracing_subscriber::Layer;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[async_trait]
trait Stoppable {
    async fn stop(self);
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> eyre::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        prev_hook(panic_info);
    }));

    let (reloadable_tracing_layer, reload_handle) = reload::Layer::new(None);

    let (log_layer, _worker_guard) = log_layer();

    tracing_subscriber::registry()
        .with(env_filter())
        .with(log_layer)
        .with(reloadable_tracing_layer)
        .init();

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
            tracing::info!(
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

    if let Some(true) = config.print_config {
        let config = toml::to_string_pretty(&config)?;
        tracing::info!("Loaded config:\n{}", config);
    }

    let resolved_config = config.clone().resolve()?;

    let acquire_strategy = if resolved_config.dev_mode_config.enable {
        AcquireStrategy::RoundRobin
    } else {
        AcquireStrategy::Strict
    };

    let cpu_topology = HwlocCPUTopology::new()?;

    let thread_pinner = Arc::new(cpu_utils::pinning::DEFAULT);

    let (core_distributor, core_distributor_persistence_task) =
        PersistentCoreDistributor::from_path(
            resolved_config.dir_config.core_state_path.clone(),
            resolved_config.node_config.system_cpu_count,
            resolved_config.node_config.cpus_range.clone(),
            acquire_strategy,
            &cpu_topology,
        )?;
    let system_cpu_cores_assignment = core_distributor.get_system_cpu_assignment();

    let builder_thread_pinner = thread_pinner.clone();
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    // worker thread count should be equal assigned logical CPU count
    // because it is the optimal count of worker threads in Tokio runtime
    // also we pin these threads to the assigned cores to prevent influence threads on each other
    builder.worker_threads(system_cpu_cores_assignment.logical_core_ids.len());
    builder.on_thread_start(move || {
        builder_thread_pinner
            .pin_current_thread_to_cpuset(&system_cpu_cores_assignment.logical_core_ids);
    });

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
            core_distributor_persistence_task.run().await;

            let key_pair = resolved_config.node_config.root_key_pair.clone();
            let base64_key_pair = base64.encode(key_pair.public().to_vec());
            let peer_id = to_peer_id(&key_pair.into());

            if let Some(config) = &config.tracing {
                let layer = tracing_layer(config, peer_id, VERSION)?;
                reload_handle.modify(move |tracing_layer| *tracing_layer = Some(layer.boxed()))?;
            }

            log::info!("node public key = {}", base64_key_pair);
            log::info!("node server peer id = {}", peer_id);

            let interpreter_path =
                to_abs_path(resolved_config.dir_config.air_interpreter_path.clone());
            write_default_air_interpreter(&interpreter_path)?;
            log::info!("AIR interpreter: {:?}", interpreter_path);

            let fluence =
                start_fluence(resolved_config, core_distributor, thread_pinner, peer_id).await?;
            log::info!("Fluence has been successfully started.");

            signal::ctrl_c().await.expect("Failed to listen for event");
            log::info!("Shutting down...");

            fluence.stop().await;
            Ok(())
        })
}

// NOTE: to stop Fluence just call Stoppable::stop()
async fn start_fluence(
    config: ResolvedConfig,
    core_distributor: Arc<dyn CoreDistributor>,
    thread_pinner: Arc<dyn ThreadPinner>,
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
        core_distributor,
        thread_pinner,
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
        cancellation_token: CancellationToken,
        node_exit_outlet: oneshot::Sender<()>,
    }

    #[async_trait]
    impl Stoppable for Fluence {
        async fn stop(self) {
            self.node_exit_outlet
                .send(())
                .expect("failed to stop node through exit outlet");
            self.cancellation_token.cancelled().await
        }
    }

    Ok(Fluence {
        node_exit_outlet: started_node.exit_outlet,
        cancellation_token: started_node.cancellation_token,
    })
}

fn vm_config(config: &ResolvedConfig) -> VmConfig {
    VmConfig::new(
        to_peer_id(&config.root_key_pair.clone().into()),
        config.dir_config.air_interpreter_path.clone(),
        config
            .node_config
            .avm_config
            .aquavm_heap_size_limit
            .map(|byte_size| byte_size.as_u64()),
        config
            .node_config
            .avm_config
            .air_size_limit
            .map(|byte_size| byte_size.as_u64()),
        config
            .node_config
            .avm_config
            .particle_size_limit
            .map(|byte_size| byte_size.as_u64()),
        config
            .node_config
            .avm_config
            .call_result_size_limit
            .map(|byte_size| byte_size.as_u64()),
        config.node_config.avm_config.hard_limit_enabled,
    )
}
