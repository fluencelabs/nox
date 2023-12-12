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
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::Instrument;

use fluence_libp2p::PeerId;
use health::HealthCheckRegistry;
use key_manager::KeyManager;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::Particle;
use peer_metrics::{ParticleExecutorMetrics, VmPoolMetrics};

use crate::aqua_runtime::AquaRuntime;
use crate::command::Command;
use crate::command::Command::{AddService, Ingest, RemoveService};
use crate::error::AquamarineApiError;
use crate::particle_effects::RoutingEffects;
use crate::vm_pool::VmPool;
use crate::{DatastoreConfig, ParticleDataStore, Plumber, VmPoolConfig};

pub type EffectsChannel = mpsc::Sender<Result<RoutingEffects, AquamarineApiError>>;

pub struct AquamarineBackend<RT: AquaRuntime, F> {
    inlet: mpsc::Receiver<Command>,
    plumber: Plumber<RT, F>,
    out: EffectsChannel,
    host_peer_id: PeerId,
    data_store: Arc<ParticleDataStore>,
}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> AquamarineBackend<RT, F> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: VmPoolConfig,
        runtime_config: RT::Config,
        datastore_config: DatastoreConfig,
        builtins: F,
        out: EffectsChannel,
        plumber_metrics: Option<ParticleExecutorMetrics>,
        vm_pool_metrics: Option<VmPoolMetrics>,
        health_registry: Option<&mut HealthCheckRegistry>,
        key_manager: KeyManager,
    ) -> eyre::Result<(Self, AquamarineApi)> {
        // TODO: make `100` configurable
        let (outlet, inlet) = mpsc::channel(100);
        let sender = AquamarineApi::new(outlet, config.execution_timeout);

        let data_store = ParticleDataStore::new(
            datastore_config.particles_dir,
            datastore_config.particles_vault_dir,
            datastore_config.particles_anomaly_dir,
        );
        let data_store: Arc<ParticleDataStore> = Arc::new(data_store);
        let vm_pool = VmPool::new(
            config.pool_size,
            runtime_config,
            vm_pool_metrics,
            health_registry,
        );
        let host_peer_id = key_manager.get_host_peer_id();
        let plumber = Plumber::new(
            vm_pool,
            data_store.clone(),
            builtins,
            plumber_metrics,
            key_manager,
        );
        let this = Self {
            inlet,
            plumber,
            out,
            host_peer_id,
            data_store,
        };

        Ok((this, sender))
    }

    pub fn poll(&mut self, cx: &mut std::task::Context<'_>) -> Poll<()> {
        let mut wake = false;

        // check if there are new particles
        loop {
            match self.inlet.poll_recv(cx) {
                Poll::Ready(Some(Ingest { particle, function })) => {
                    wake = true;
                    // set new particle to be executed
                    // every particle that comes from the connection pool first executed on the host peer id
                    self.plumber.ingest(particle, function, self.host_peer_id);
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

    pub fn start(mut self) -> JoinHandle<()> {
        let data_store = self.data_store.clone();
        let mut stream = futures::stream::poll_fn(move |cx| self.poll(cx).map(|_| Some(()))).fuse();
        let result = tokio::task::Builder::new()
            .name("AVM")
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
    pub fn execute(
        self,
        particle: Particle,
        function: Option<ServiceFunction>,
    ) -> impl Future<Output = Result<(), AquamarineApiError>> {
        let particle_id = particle.id.clone();
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
    }
}
