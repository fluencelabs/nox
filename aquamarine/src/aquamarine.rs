/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use futures::StreamExt;
use marine_wasmtime_backend::WasmtimeWasmBackend;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{instrument, Instrument};

use health::HealthCheckRegistry;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::ExtendedParticle;
use particle_services::{PeerScope, WasmBackendConfig};
use peer_metrics::{ParticleExecutorMetrics, VmPoolMetrics};
use workers::{Event, KeyStorage, PeerScopes, Receiver, Workers};

use crate::command::Command;
use crate::command::Command::{AddService, Ingest, RemoveService};
use crate::error::AquamarineApiError;
use crate::vm_pool::VmPool;
use crate::{
    AquaRuntime, DataStoreConfig, ParticleDataStore, Plumber, RemoteRoutingEffects, VmPoolConfig,
};

pub type EffectsChannel = mpsc::Sender<Result<RemoteRoutingEffects, AquamarineApiError>>;

pub struct AquamarineBackend<RT: AquaRuntime, F> {
    inlet: mpsc::Receiver<Command>,
    worker_events: Receiver<Event>,
    plumber: Plumber<RT, F>,
    out: EffectsChannel,
    data_store: Arc<ParticleDataStore>,
}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> AquamarineBackend<RT, F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: VmPoolConfig,
        vm_config: RT::Config,
        avm_wasm_backend_config: WasmBackendConfig,
        data_store_config: DataStoreConfig,
        builtins: F,
        out: EffectsChannel,
        plumber_metrics: Option<ParticleExecutorMetrics>,
        vm_pool_metrics: Option<VmPoolMetrics>,
        health_registry: Option<&mut HealthCheckRegistry>,
        workers: Arc<Workers>,
        key_storage: Arc<KeyStorage>,
        scopes: PeerScopes,
        worker_events: Receiver<Event>,
    ) -> eyre::Result<(Self, AquamarineApi)> {
        // TODO: make `100` configurable
        let (outlet, inlet) = mpsc::channel(100);
        let sender = AquamarineApi::new(outlet, config.execution_timeout);

        let data_store = ParticleDataStore::new(
            data_store_config.particles_dir,
            data_store_config.particles_vault_dir,
            data_store_config.particles_anomaly_dir,
        );
        let data_store: Arc<ParticleDataStore> = Arc::new(data_store);
        let avm_wasm_backend = WasmtimeWasmBackend::new(avm_wasm_backend_config.into())?;

        let vm_pool = VmPool::new(
            config.pool_size,
            vm_config.clone(),
            vm_pool_metrics,
            health_registry,
            avm_wasm_backend.clone(),
        );
        let plumber = Plumber::new(
            vm_config,
            vm_pool,
            data_store.clone(),
            builtins,
            plumber_metrics,
            workers,
            key_storage,
            scopes,
            avm_wasm_backend,
        );
        let this = Self {
            inlet,
            worker_events,
            plumber,
            out,
            data_store,
        };

        Ok((this, sender))
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let mut wake = self.process_worker_events();

        // check if there are new particles
        loop {
            match self.inlet.poll_recv(cx) {
                Poll::Ready(Some(Ingest { particle, function })) => {
                    wake = true;
                    let span = tracing::info_span!(parent: particle.span.as_ref(), "Aquamarine::poll::ingest");
                    let _guard = span.entered();
                    // set new particle to be executed
                    // every particle that comes from the connection pool first executed on the host peer id
                    self.plumber.ingest(particle, function, PeerScope::Host);
                }
                Poll::Ready(Some(AddService {
                    service,
                    functions,
                    fallback,
                })) => self.plumber.add_service(service, functions, fallback),

                Poll::Ready(Some(RemoveService { service })) => {
                    self.plumber.remove_service(service)
                }

                Poll::Pending | Poll::Ready(None) => break,
            }
        }

        // check if there are executed particles
        while let Poll::Ready(effects) = self.plumber.poll(cx) {
            wake = true;
            // send results back
            let sent = self.out.try_send(effects);
            if let Err(err) = sent {
                log::error!("Aquamarine effects outlet has died: {}", err);
            }
        }

        if wake {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn process_worker_events(&mut self) -> bool {
        let mut wake = false;
        loop {
            let res = self.worker_events.try_recv();
            match res {
                Ok(event) => match event {
                    Event::WorkerCreated {
                        worker_id,
                        thread_count,
                    } => {
                        wake = true;
                        self.plumber.create_worker_pool(worker_id, thread_count);
                    }
                    Event::WorkerRemoved { worker_id } => {
                        self.plumber.remove_worker_pool(worker_id);
                    }
                },
                Err(_) => {
                    break;
                }
            }
        }
        wake
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let data_store = self.data_store.clone();
        let mut stream = futures::stream::poll_fn(move |cx| self.poll(cx).map(|_| Some(()))).fuse();
        let result = tokio::task::Builder::new()
            .name("Aquamarine")
            .spawn(
                async move {
                    data_store
                        .initialize()
                        .await
                        .expect("Could not initialize data store");
                    loop {
                        stream.next().await;
                    }
                }
                .in_current_span(),
            )
            .expect("Could not spawn task");

        result
    }
}

#[derive(Clone)]
pub struct AquamarineApi {
    outlet: mpsc::Sender<Command>,
    #[allow(dead_code)]
    execution_timeout: Duration,
}

impl AquamarineApi {
    pub fn new(outlet: mpsc::Sender<Command>, execution_timeout: Duration) -> Self {
        Self {
            outlet,
            execution_timeout,
        }
    }

    /// Send particle to the interpreters pool
    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub fn execute(
        self,
        particle: ExtendedParticle,
        function: Option<ServiceFunction>,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        let particle_id = particle.particle.id.clone();
        self.send_command(Ingest { particle, function }, Some(particle_id))
    }

    pub fn add_service(
        self,
        service: String,
        functions: HashMap<String, ServiceFunction>,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        self.send_command(
            AddService {
                service,
                functions,
                fallback: None,
            },
            None,
        )
    }

    pub fn remove_service(
        self,
        service: String,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        self.send_command(RemoveService { service }, None)
    }

    fn send_command(
        self,
        command: Command,
        particle_id: Option<String>,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        use AquamarineApiError::*;

        let interpreters = self.outlet;

        async move {
            let sent = interpreters.send(command).await;

            sent.map_err(|_err| {
                log::error!("Aquamarine outlet died!");
                AquamarineDied { particle_id }
            })
        }
        .in_current_span()
    }
}
