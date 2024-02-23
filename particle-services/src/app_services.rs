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
use std::ops::Deref;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};

use derivative::Derivative;
use fluence_app_service::{
    AppService, AppServiceConfig, AppServiceError, CallParameters, MarineConfig, MarineError,
    MarineWASIConfig, ModuleDescriptor, SecurityTetraplet, ServiceInterface,
};
use humantime_serde::re::humantime::format_duration as pretty;
use parking_lot::{Mutex, RwLock, RwLockUpgradableReadGuard};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JValue};
use tokio::runtime::Handle;
use tokio_util::context::TokioContext;

use fluence_libp2p::PeerId;
use health::HealthCheckRegistry;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_execution::{FunctionOutcome, ParticleParams, ParticleVault};
use particle_modules::ModuleRepository;
use peer_metrics::{
    ServiceCallStats, ServiceMemoryStat, ServiceType as MetricServiceType, ServicesMetrics,
    ServicesMetricsBuiltin,
};
use server_config::ServicesConfig;
use types::peer_scope::PeerScope;
use uuid_utils::uuid;
use workers::{PeerScopes, WorkerId, Workers};

use crate::error::ServiceError;
use crate::error::ServiceError::{AliasAsServiceId, Forbidden, NoSuchAlias};
use crate::health::PersistedServiceHealth;
use crate::persistence::{load_persisted_services, remove_persisted_service, PersistedService};
use crate::ServiceError::{
    FailedToCreateDirectory, ForbiddenAlias, ForbiddenAliasRoot, ForbiddenAliasWorker,
    InternalError, NoSuchService,
};

type ServiceId = String;
type ServiceAlias = String;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Service,
    Spell,
}

impl ServiceType {
    pub fn is_spell(&self) -> bool {
        matches!(self, ServiceType::Spell)
    }
}

#[derive(Debug)]
pub struct ServiceInfo {
    pub id: String,
    pub blueprint_id: String,
    pub service_type: ServiceType,
    pub owner_id: PeerId,
    pub aliases: Vec<ServiceAlias>,
    pub peer_scope: PeerScope,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Service {
    #[derivative(Debug(format_with = "fmt_service"))]
    pub service: Mutex<AppService>,
    pub service_id: String,
    pub blueprint_id: String,
    pub service_type: ServiceType,
    pub owner_id: PeerId,
    pub aliases: RwLock<Vec<ServiceAlias>>,
    pub peer_scope: PeerScope,
}

impl Service {
    pub fn new(
        service: Mutex<AppService>,
        service_id: String,
        blueprint_id: String,
        service_type: ServiceType,
        owner_id: PeerId,
        aliases: Vec<ServiceAlias>,
        peer_scope: PeerScope,
    ) -> Self {
        Self {
            service,
            service_id,
            blueprint_id,
            service_type,
            owner_id,
            aliases: RwLock::new(aliases),
            peer_scope,
        }
    }

    pub fn remove_alias(&self, alias: &str) {
        let mut aliases = self.aliases.write();
        if let Some(pos) = aliases.iter().position(|x| *x == alias) {
            aliases.remove(pos);
        }
    }

    pub fn add_alias(&self, alias: String) {
        self.aliases.write().push(alias);
    }

    pub fn get_info(&self, id: &str) -> ServiceInfo {
        ServiceInfo {
            id: id.to_string(),
            blueprint_id: self.blueprint_id.clone(),
            service_type: self.service_type.clone(),
            owner_id: self.owner_id,
            aliases: self.aliases.read().clone(),
            peer_scope: self.peer_scope,
        }
    }
}

impl Deref for Service {
    type Target = Mutex<AppService>;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

fn fmt_service(
    _: &Mutex<AppService>,
    f: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    f.debug_struct("Mutex<AppService>").finish()
}

#[derive(Serialize)]
pub struct VmDescriptor<'a> {
    interface: ServiceInterface,
    blueprint_id: &'a str,
    service_id: &'a str,
    owner_id: &'a str,
}

#[derive(Derivative)]
#[derivative(Debug, Clone, Default)]
struct Services {
    services: Arc<RwLock<HashMap<ServiceId, Arc<Service>>>>,
    aliases: Arc<RwLock<HashMap<ServiceAlias, ServiceId>>>,
}

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct ParticleAppServices {
    config: ServicesConfig,
    // TODO: move vault to Plumber or Actor
    pub vault: ParticleVault,
    root_services: Services,
    #[derivative(Debug = "ignore")]
    root_runtime_handle: Handle,
    worker_services: Arc<RwLock<HashMap<WorkerId, Services>>>,
    modules: ModuleRepository,
    #[derivative(Debug = "ignore")]
    workers: Arc<Workers>,
    #[derivative(Debug = "ignore")]
    scopes: PeerScopes,
    pub metrics: Option<ServicesMetrics>,
    health: Option<PersistedServiceHealth>,
}

fn resolve_alias(services: &Services, alias: &String, particle_id: &str) -> Option<ServiceId> {
    if alias == "spell" || alias == "self" {
        if let Some(spell_id) = ParticleParams::get_spell_id(particle_id) {
            return Some(spell_id);
        }
    }

    services.aliases.read().get(alias).cloned()
}

fn get_service(
    services: &HashMap<ServiceId, Arc<Service>>,
    peer_scope: PeerScope,
    service_id: ServiceId,
) -> Result<Arc<Service>, ServiceError> {
    let service = services
        .get(&service_id)
        .ok_or(NoSuchService(service_id.to_string(), peer_scope))?;

    Ok(service.clone())
}

impl ParticleAppServices {
    pub fn new(
        config: ServicesConfig,
        modules: ModuleRepository,
        metrics: Option<ServicesMetrics>,
        health_registry: Option<&mut HealthCheckRegistry>,
        workers: Arc<Workers>,
        scope: PeerScopes,
    ) -> Self {
        let vault = ParticleVault::new(config.particles_vault_dir.clone());
        let root_runtime_handle = Handle::current();

        let health = health_registry.map(|registry| {
            let persisted_services = PersistedServiceHealth::new();
            registry.register("persisted_services", persisted_services.clone());
            persisted_services
        });
        Self {
            config,
            vault,
            root_services: <_>::default(),
            root_runtime_handle,
            worker_services: <_>::default(),
            modules,
            workers,
            scopes: scope,
            metrics,
            health,
        }
    }

    pub async fn create_service(
        &self,
        peer_scope: PeerScope,
        service_type: ServiceType,
        blueprint_id: String,
        owner_id: PeerId,
    ) -> Result<String, ServiceError> {
        let service_id = uuid::Uuid::new_v4().to_string();

        let runtime_handle = match peer_scope {
            PeerScope::WorkerId(worker_id) => self
                .workers
                .get_handle(worker_id)
                .ok_or(ServiceError::WorkerNotFound { worker_id })?,
            PeerScope::Host => self.root_runtime_handle.clone(),
        };

        let fut = async {
            self.create_service_inner(
                service_type,
                blueprint_id,
                owner_id,
                peer_scope,
                service_id.clone(),
                vec![],
            )
            .await
        };

        TokioContext::new(fut, runtime_handle).await?;

        Ok(service_id)
    }

    pub fn service_exists(&self, peer_scope: &PeerScope, service_id: &str) -> bool {
        let services = self.get_services(peer_scope);
        match services {
            Ok(services) => services.services.read().get(service_id).is_some(),
            Err(_) => false,
        }
    }

    pub fn get_service_info(
        &self,
        peer_scope: PeerScope,
        service_id_or_alias: String,
        particle_id: &str,
    ) -> Result<ServiceInfo, ServiceError> {
        let (service, service_id) =
            self.get_service(peer_scope, service_id_or_alias, particle_id)?;

        Ok(service.get_info(&service_id))
    }

    pub async fn remove_services(&self, peer_scope: PeerScope) -> Result<(), ServiceError> {
        let services = self.get_services(&peer_scope)?;
        let service_ids: Vec<ServiceId> = services.services.read().keys().cloned().collect();

        for srv_id in service_ids {
            //TODO: can be parallelized
            if let Err(err) =
                remove_persisted_service(&self.config.services_dir, srv_id.clone()).await
            {
                tracing::warn!(
                    "Error while removing persisted service for {service_id}: {:?}",
                    err,
                    service_id = srv_id,
                )
            }
        }
        let services = self.get_services(&peer_scope)?;

        let mut aliases = services.aliases.write();
        let mut services = services.services.write();

        aliases.clear();
        services.clear();

        Ok(())
    }

    pub async fn remove_service(
        &self,
        peer_scope: PeerScope,
        particle_id: &str,
        service_id_or_alias: &str,
        init_peer_id: PeerId,
        allow_remove_spell: bool,
    ) -> Result<(), ServiceError> {
        let removal_start_time = Instant::now();
        let service_id = {
            let (service, service_id) =
                self.get_service(peer_scope, service_id_or_alias.to_string(), particle_id)?;

            // tmp hack to forbid spell removal via srv.remove
            if service.service_type.is_spell() && !allow_remove_spell {
                return Err(Forbidden {
                    user: init_peer_id,
                    function: "remove_service",
                    reason: "cannot remove a spell",
                });
            } else if !service.service_type.is_spell() && allow_remove_spell {
                return Err(Forbidden {
                    user: init_peer_id,
                    function: "remove_spell",
                    reason: "the service isn't a spell",
                });
            }

            // TODO: HACK:
            //  What a mess.
            //  service.owner_id has created the service, so can remove. that's OK.
            //  management_peer_id is the node admin, can remove any service. that's OK.
            //  service.worker_id is the worker itself, so can remove. that's OK.

            let service_worker_id: PeerId = self.scopes.to_peer_id(peer_scope);

            if service_worker_id != init_peer_id
                && service.owner_id != init_peer_id
                && !self.scopes.is_management(init_peer_id)
            {
                return Err(Forbidden {
                    user: init_peer_id,
                    function: "remove_service",
                    reason: "only creator can remove service",
                });
            }

            service_id
        };

        if let Err(err) =
            remove_persisted_service(&self.config.services_dir, service_id.clone()).await
        {
            tracing::warn!(
                "Error while removing persisted service for {}: {:?}",
                service_id,
                err
            )
        }
        let services = self.get_services(&peer_scope)?;
        let mut aliases = services.aliases.write();
        let mut services = services.services.write();
        let service = services.remove(service_id.as_str()).unwrap();
        let service_aliases = service.aliases.read();
        for alias in service_aliases.iter() {
            aliases.remove(alias.as_str());
        }
        let service_type = self.get_service_type(&service, &service.peer_scope);

        let removal_end_time = removal_start_time.elapsed().as_secs();
        if let Some(metrics) = self.metrics.as_ref() {
            metrics.observe_removed(service_type, removal_end_time as f64);
        }

        Ok(())
    }

    pub fn call_service(
        &self,
        function_args: Args,
        particle: ParticleParams,
        create_vault: bool,
    ) -> FunctionOutcome {
        let peer_scope = particle.peer_scope;
        let timestamp = particle.timestamp;

        let service = self.get_service(peer_scope, function_args.service_id.clone(), &particle.id);

        let (service, service_id) = match service {
            Ok(found) => found,
            // If service is not found, report it
            Err(_err) => {
                return FunctionOutcome::NotDefined {
                    args: function_args,
                    params: particle,
                };
            }
        };

        // TODO: figure out how to check for builtins (like registry, aqua-ipfs)
        // if service.worker_id != worker_id {
        //     return FunctionOutcome::Err(JError::from(
        //         ServiceError::CallServiceFailedWrongWorker {
        //             service_id,
        //             worker_id,
        //         },
        //     ));
        // }
        // Metrics collection are enables for services with aliases which are installed on root worker or worker spells.
        let service_type = self.get_service_type(service.as_ref(), &peer_scope);

        // TODO: move particle vault creation to aquamarine::particle_functions
        if create_vault {
            self.vault.create(&particle)?;
        }

        let call_parameters_worker_id = self.scopes.to_peer_id(peer_scope);

        let params = CallParameters {
            particle: particle.to_particle_parameters(),
            service_id: service_id.clone(),
            service_creator_peer_id: service.owner_id.to_string(),
            host_id: self.scopes.get_host_peer_id().to_string(),
            worker_id: call_parameters_worker_id.to_string(),
            tetraplets: function_args
                .tetraplets
                .into_iter()
                .map(|sts| {
                    sts.into_iter()
                        .map(|st| SecurityTetraplet {
                            peer_pk: st.peer_pk,
                            service_id: st.service_id,
                            function_name: st.function_name,
                            lens: st.json_path,
                        })
                        .collect()
                })
                .collect(),
        };
        let function_name = function_args.function_name;

        let lock_acquire_start = Instant::now();
        let mut service = service.lock();
        let old_memory = service.module_memory_stats();
        let old_mem_usage = ServicesMetricsBuiltin::get_used_memory(&old_memory);
        // TODO: set execution timeout https://github.com/fluencelabs/fluence/issues/1212
        let call_time_start = Instant::now();
        let result = service
            .call(
                function_name.clone(),
                JValue::Array(function_args.function_args),
                params,
            )
            .map_err(|e| {
                if let Some(metrics) = self.metrics.as_ref() {
                    let stats = ServiceCallStats::Fail { timestamp };
                    // If the called function is unknown we don't want to save info
                    // about it in a separate entry.
                    let function_name = if is_unknown_function(&e) {
                        None
                    } else {
                        Some(function_name.clone())
                    };
                    metrics.observe_service_state_failed(
                        service_id.clone(),
                        function_name,
                        service_type.clone(),
                        stats,
                    );
                }
                ServiceError::Engine(e)
            })?;

        if let Some(metrics) = self.metrics.as_ref() {
            let call_time_sec = call_time_start.elapsed().as_secs_f64();
            let lock_wait_time_sec = lock_acquire_start.elapsed().as_secs_f64();
            let new_memory = service.module_memory_stats();
            let new_memory_usage = ServicesMetricsBuiltin::get_used_memory(&new_memory);

            let memory_delta_bytes = new_memory_usage - old_mem_usage;
            let stats = ServiceCallStats::Success {
                memory_delta_bytes: memory_delta_bytes as f64,
                call_time_sec,
                lock_wait_time_sec,
                timestamp,
            };

            metrics.observe_service_state(
                service_id,
                function_name,
                service_type,
                ServiceMemoryStat::new(&new_memory),
                stats,
            );
        }

        FunctionOutcome::Ok(result)
    }

    // TODO: is it safe?
    #[allow(clippy::too_many_arguments)]
    pub fn call_function(
        &self,
        peer_scope: PeerScope,
        service_id: &str,
        function_name: &str,
        function_args: Vec<JValue>,
        particle_id: Option<String>,
        init_peer_id: PeerId,
        particle_ttl: Duration,
    ) -> FunctionOutcome {
        let args = Args {
            service_id: service_id.to_string(),
            function_name: function_name.to_string(),
            function_args,
            tetraplets: vec![],
        };

        let particle = ParticleParams {
            id: particle_id.unwrap_or(uuid()),
            init_peer_id,
            peer_scope,
            timestamp: now_ms() as u64,
            ttl: particle_ttl.as_millis() as u32,
            script: "".to_string(),
            signature: vec![],
            // TODO: Is it safe?
            token: "host_call".to_string(),
        };

        self.call_service(args, particle, false)
    }

    async fn add_alias_inner(
        &self,
        alias: String,
        peer_scope: PeerScope,
        service_id: ServiceId,
    ) -> Result<(), ServiceError> {
        let service = {
            let services = self.get_or_create_services(peer_scope);
            let mut aliases_service_id_mapping = services.aliases.write();
            let services_id_mapping = services.services.write();

            let service = get_service(&services_id_mapping, peer_scope, service_id.clone())?;
            service.add_alias(alias.clone());
            aliases_service_id_mapping.insert(alias, service_id);
            PersistedService::from_service(service.as_ref())
        };

        service.persist(&self.config.services_dir).await
    }

    fn get_or_create_services(&self, peer_scope: PeerScope) -> Services {
        match peer_scope {
            PeerScope::WorkerId(worker_id) => self.get_or_create_worker_services(worker_id),
            PeerScope::Host => self.root_services.clone(),
        }
    }

    fn get_services(&self, peer_scope: &PeerScope) -> Result<Services, ServiceError> {
        match peer_scope {
            PeerScope::WorkerId(worker_id) => {
                let worker_services = self.worker_services.read();
                let services =
                    worker_services
                        .get(worker_id)
                        .ok_or(ServiceError::WorkerNotFound {
                            worker_id: *worker_id,
                        })?;
                Ok(services.clone())
            }
            PeerScope::Host => Ok(self.root_services.clone()),
        }
    }

    pub fn get_service(
        &self,
        peer_scope: PeerScope,
        id_or_alias: String,
        particle_id: &str,
    ) -> Result<(Arc<Service>, String), ServiceError> {
        let services = self.get_services(&peer_scope)?;
        let services_id_mapping = services.services.read();

        // retrieve service by service id
        let service = get_service(&services_id_mapping, peer_scope, id_or_alias.clone()).ok();

        if let Some(service) = service {
            return Ok((service, id_or_alias));
        }

        // retrieve service by alias
        let resolved_id = resolve_alias(&services, &id_or_alias, particle_id)
            .ok_or(NoSuchService(id_or_alias.clone(), peer_scope))?;

        let service = get_service(&services_id_mapping, peer_scope, resolved_id.clone())?;

        Ok((service, resolved_id))
    }

    fn get_service_id(&self, peer_scope: PeerScope, alias: &str) -> Option<ServiceId> {
        let services = self.get_services(&peer_scope);
        match services {
            Ok(services) => services.aliases.read().get(alias).cloned(),
            Err(_) => None,
        }
    }

    async fn remove_alias(
        &self,
        alias: String,
        peer_scope: PeerScope,
        service_id: &str,
    ) -> Result<(), ServiceError> {
        let service = {
            let services = self.get_or_create_services(peer_scope);
            let services_id_mapping = services.services.write();
            let service = get_service(&services_id_mapping, peer_scope, service_id.to_string())?;
            service.remove_alias(&alias);
            PersistedService::from_service(service.as_ref())
        };

        service.persist(&self.config.services_dir).await
    }

    pub async fn add_alias(
        &self,
        peer_scope: PeerScope,
        alias: String,
        service_id: String,
        init_peer_id: PeerId,
    ) -> Result<(), ServiceError> {
        let is_management = self.scopes.is_management(init_peer_id);

        if !is_management {
            match peer_scope {
                PeerScope::WorkerId(worker_id) => {
                    let worker_creator = self
                        .workers
                        .get_worker_creator(worker_id)
                        .map_err(|e| InternalError(format!("{e:?}")))?;

                    if init_peer_id != worker_creator && init_peer_id != worker_id.into() {
                        return Err(ForbiddenAliasWorker(init_peer_id));
                    }
                }
                PeerScope::Host => {
                    if init_peer_id != self.scopes.get_host_peer_id() {
                        return Err(ForbiddenAliasRoot(init_peer_id));
                    }
                }
            }
        }

        // alias can't be equal to any existent service id
        if self.service_exists(&peer_scope, &alias) {
            return Err(AliasAsServiceId(alias));
        }

        if alias == "spell" || alias == "self" {
            return Err(ForbiddenAlias(alias));
        }

        if !self.service_exists(&peer_scope, &service_id) {
            return Err(NoSuchService(service_id, peer_scope));
        }

        let prev_srv_id = self.get_service_id(peer_scope, &alias);
        if let Some(srv_id) = prev_srv_id {
            self.remove_alias(alias.clone(), peer_scope, &srv_id)
                .await?;
        }

        self.add_alias_inner(alias.clone(), peer_scope, service_id.clone())
            .await?;

        Ok(())
    }

    pub fn resolve_alias(
        &self,
        peer_scope: PeerScope,
        alias: String,
        particle_id: &str,
    ) -> Result<String, ServiceError> {
        let services = self.get_or_create_services(peer_scope);
        resolve_alias(&services, &alias, particle_id).ok_or_else(|| NoSuchAlias(alias, peer_scope))
    }

    pub fn to_service_id(
        &self,
        peer_scope: PeerScope,
        service_id_or_alias: String,
        particle_id: &str,
    ) -> Result<String, ServiceError> {
        let (_, service_id) = self.get_service(peer_scope, service_id_or_alias, particle_id)?;
        Ok(service_id)
    }

    pub fn get_service_owner(
        &self,
        peer_scope: PeerScope,
        id_or_alias: String,
        particle_id: &str,
    ) -> Result<PeerId, ServiceError> {
        let (service, _) = self.get_service(peer_scope, id_or_alias, particle_id)?;

        Ok(service.owner_id)
    }

    pub fn check_service_worker_id(
        &self,
        peer_scope: PeerScope,
        id_or_alias: String,
        particle_id: &str,
    ) -> Result<(), ServiceError> {
        let (service, _) = self.get_service(peer_scope, id_or_alias.clone(), particle_id)?;

        if service.peer_scope != peer_scope {
            Err(ServiceError::CallServiceFailedWrongWorker {
                service_id: id_or_alias,
                peer_scope,
            })
        } else {
            Ok(())
        }
    }

    pub fn get_interface(
        &self,
        peer_scope: PeerScope,
        service_id: String,
        particle_id: &str,
    ) -> Result<JValue, ServiceError> {
        let (service, _) = self.get_service(peer_scope, service_id, particle_id)?;

        Ok(self.modules.get_facade_interface(&service.blueprint_id)?)
    }

    pub fn list_services_all(&self) -> Vec<ServiceInfo> {
        let root_info: Vec<ServiceInfo> = self
            .root_services
            .services
            .read()
            .iter()
            .map(|(id, service)| service.get_info(id))
            .collect();

        let mut result: Vec<ServiceInfo> = self
            .worker_services
            .read()
            .values()
            .flat_map(|services| {
                let services = services.services.read();
                services
                    .iter()
                    .map(|(id, service)| service.get_info(id))
                    .collect::<Vec<ServiceInfo>>()
            })
            .collect();
        result.extend(root_info);
        result
    }

    pub fn list_services(&self, peer_scope: PeerScope) -> Vec<ServiceInfo> {
        let services = self.get_services(&peer_scope);
        match services {
            Ok(services) => services
                .services
                .read()
                .iter()
                .map(|(id, srv)| srv.get_info(id))
                .collect(),
            Err(_) => {
                vec![]
            }
        }
    }

    pub fn get_service_mem_stats(
        &self,
        peer_scope: PeerScope,
        service_id: String,
        particle_id: &str,
    ) -> Result<Vec<JValue>, JError> {
        let (service, _) = self.get_service(peer_scope, service_id, particle_id)?;

        let lock = service.service.lock();
        let stats = lock.module_memory_stats();
        let stats = stats
            .modules
            .into_iter()
            .map(|stat| {
                json!({
                    "name": stat.name,
                    "memory_size_bytes": stat.memory_size,
                })
            })
            .collect();

        // TODO: report service memory limit
        // TODO: report allocation rejects (cleared after each call, optional value but always Some on wasmtime)
        Ok(stats)
    }

    pub async fn create_persisted_services(&mut self) -> eyre::Result<()> {
        let services = load_persisted_services(&self.config.services_dir).await?;
        let loaded_service_count = services.len();
        if let Some(h) = self.health.as_mut() {
            h.start_creation()
        }

        let mut created_service_count = 0;
        for (service, _) in services {
            let start = Instant::now();
            // If the service_type doesn't set in PersistedService, will try to find out if it's a spell by blueprint name
            // This is mostly done for migration from the old detection method to the new.
            let service_type = service.service_type.unwrap_or_else(|| {
                let is_spell: Option<_> = try {
                    let blueprint_name = self
                        .modules
                        .get_blueprint_from_cache(&service.blueprint_id)
                        .ok()?
                        .name;
                    blueprint_name == "spell"
                };
                if is_spell.unwrap_or(false) {
                    ServiceType::Spell
                } else {
                    ServiceType::Service
                }
            });
            let result = self
                .create_service_inner(
                    service_type,
                    service.blueprint_id,
                    service.owner_id,
                    service.peer_scope,
                    service.service_id.clone(),
                    service.aliases.clone(),
                )
                .await;
            let replaced = match result {
                Ok(replaced) => replaced,
                Err(err) => {
                    #[rustfmt::skip]
                    tracing::warn!("Error creating service for persisted service {}: {:#?}", service.service_id, err);
                    continue;
                }
            };

            match service.peer_scope {
                PeerScope::WorkerId(worker_id) => {
                    let services = self.get_or_create_worker_services(worker_id);
                    let mut aliases = services.aliases.write();
                    for alias in service.aliases.iter() {
                        let old = aliases.insert(alias.clone(), service.service_id.clone());
                        if let Some(old) = old {
                            tracing::warn!(
                                "Alias `{}` is the same for {} and {}",
                                alias,
                                old,
                                service.service_id
                            );
                        }
                    }
                }
                PeerScope::Host => {
                    let mut aliases = self.root_services.aliases.write();
                    for alias in service.aliases.iter() {
                        let old = aliases.insert(alias.clone(), service.service_id.clone());
                        if let Some(old) = old {
                            tracing::warn!(
                                "Alias `{}` is the same for {} and {}",
                                alias,
                                old,
                                service.service_id
                            );
                        }
                    }
                }
            };

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );
            created_service_count += 1;
            tracing::info!(
                "Persisted service {} created in {}, aliases: {:?}",
                service.service_id,
                pretty(start.elapsed()),
                service.aliases
            );
        }
        if created_service_count == loaded_service_count {
            if let Some(h) = self.health.as_mut() {
                h.finish_creation()
            }
        };
        Ok(())
    }

    async fn create_service_inner(
        &self,
        service_type: ServiceType,
        blueprint_id: String,
        owner_id: PeerId,
        peer_scope: PeerScope,
        service_id: String,
        aliases: Vec<String>,
    ) -> Result<Option<Arc<Service>>, ServiceError> {
        let creation_start_time = Instant::now();
        let service = self
            .create_app_service(blueprint_id.clone(), service_id.clone())
            .await
            .inspect_err(|_| {
                if let Some(metrics) = self.metrics.as_ref() {
                    metrics.observe_created_failed();
                }
            })?;
        let stats = service.module_memory_stats();
        let stats = ServiceMemoryStat::new(&stats);

        let service = Service::new(
            Mutex::new(service),
            service_id.clone(),
            blueprint_id,
            service_type,
            owner_id,
            aliases,
            peer_scope,
        );
        let service = Arc::new(service);
        // Save created service to disk, so it is recreated on restart
        let persisted_service = PersistedService::from_service(&service);
        persisted_service.persist(&self.config.services_dir).await?;
        let service_type = self.get_service_type(&service, &peer_scope);
        let services = self.get_or_create_services(peer_scope);
        let replaced = services
            .services
            .write()
            .insert(service_id.clone(), service);

        if let Some(m) = self.metrics.as_ref() {
            let creation_end_time = creation_start_time.elapsed().as_secs();
            m.observe_created(service_id, service_type, stats, creation_end_time as f64);
        }

        Ok(replaced)
    }

    fn get_or_create_worker_services(&self, worker_id: WorkerId) -> Services {
        let workers_services = self.worker_services.upgradable_read();
        let worker_services = workers_services.get(&worker_id);
        match worker_services {
            None => {
                let mut workers_services = RwLockUpgradableReadGuard::upgrade(workers_services);
                //we double check it, because it can be created in another thread
                let services = workers_services.get(&worker_id);
                match services {
                    None => {
                        let services = Services::default();
                        workers_services.insert(worker_id, services.clone());
                        services
                    }
                    Some(services) => services.clone(),
                }
            }
            Some(services) => services.clone(),
        }
    }

    fn inject_default_wasi(&self, module: &mut ModuleDescriptor) {
        let wasi = &mut module.config.wasi;
        if wasi.is_none() {
            *wasi = Some(MarineWASIConfig::default());
        }
    }

    async fn inject_persistent_dirs(
        &self,
        module: &mut ModuleDescriptor,
        persistent_dir: &Path,
    ) -> Result<(), ServiceError> {
        let module_dir = persistent_dir.join(&module.import_name);
        tokio::fs::create_dir_all(&module_dir)
            .await
            .map_err(|err| FailedToCreateDirectory {
                path: module_dir.clone(),
                err,
            })?;

        let wasi = module.config.wasi.as_mut().ok_or(InternalError(
            "Could not inject persistent dirs into empty WASI config".to_string(),
        ))?;
        wasi.mapped_dirs
            .insert("/storage".into(), persistent_dir.to_path_buf());
        wasi.mapped_dirs
            .insert("/storage/module".into(), module_dir);
        Ok(())
    }

    async fn inject_ephemeral_dirs(
        &self,
        module: &mut ModuleDescriptor,
        ephemeral_dir: &Path,
    ) -> Result<(), ServiceError> {
        let module_dir = ephemeral_dir.join(&module.import_name);
        tokio::fs::create_dir_all(&module_dir)
            .await
            .map_err(|err| FailedToCreateDirectory {
                path: module_dir.clone(),
                err,
            })?;

        let wasi = module.config.wasi.as_mut().ok_or(InternalError(
            "Could not inject ephemeral dirs into empty WASI config".to_string(),
        ))?;
        wasi.mapped_dirs
            .insert("/tmp".into(), ephemeral_dir.to_path_buf());
        wasi.mapped_dirs.insert("/tmp/module".into(), module_dir);
        Ok(())
    }

    async fn create_app_service(
        &self,
        blueprint_id: String,
        service_id: String,
    ) -> Result<AppService, ServiceError> {
        let persistent_dir = self.config.persistent_work_dir.join(&service_id);
        let ephemeral_dir = self.config.ephemeral_work_dir.join(&service_id);

        // TODO: introduce separate errors
        tokio::fs::create_dir_all(&persistent_dir)
            .await
            .map_err(|err| FailedToCreateDirectory {
                path: persistent_dir.clone(),
                err,
            })?;
        tokio::fs::create_dir_all(&ephemeral_dir)
            .await
            .map_err(|err| FailedToCreateDirectory {
                path: ephemeral_dir.clone(),
                err,
            })?;

        let mut modules_config = self.modules.resolve_blueprint(&blueprint_id)?;

        for module in modules_config.iter_mut() {
            self.inject_default_wasi(module);
            // SAFETY: set wasi to Some in the code before calling inject_vault
            self.vault.inject_vault(module).unwrap();
            self.inject_persistent_dirs(module, persistent_dir.as_path())
                .await?;
            self.inject_ephemeral_dirs(module, ephemeral_dir.as_path())
                .await?;
        }

        let app_config = AppServiceConfig {
            service_working_dir: persistent_dir,
            marine_config: MarineConfig {
                // TODO: add an option to set individual per-service limit
                total_memory_limit: self
                    .config
                    .default_service_memory_limit
                    .map(|bytes| bytes.as_u64()),
                modules_dir: Some(self.config.modules_dir.clone()),
                modules_config,
                default_modules_config: None,
            },
        };

        tracing::debug!(
            "Creating service {}, envs: {:?}",
            service_id,
            self.config.envs
        );

        AppService::new(app_config, service_id, self.config.envs.clone())
            .map_err(ServiceError::Engine)
    }

    fn get_service_type(&self, service: &Service, peer_scope: &PeerScope) -> MetricServiceType {
        let allowed_alias = match peer_scope {
            PeerScope::Host => service.aliases.read().first().cloned(),
            _ => {
                if service
                    .aliases
                    .read()
                    .first()
                    .map(|alias| alias == "worker-spell")
                    .unwrap_or(false)
                {
                    Some("worker-spell".to_string())
                } else {
                    None
                }
            }
        };

        if service.service_type.is_spell() {
            MetricServiceType::Spell(allowed_alias)
        } else {
            MetricServiceType::Service(allowed_alias)
        }
    }
}

fn is_unknown_function(err: &AppServiceError) -> bool {
    matches!(
        err,
        AppServiceError::MarineError(MarineError::MissingFunctionError(_))
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_app_service::{TomlMarineModuleConfig, TomlMarineNamedModuleConfig};
    use fluence_keypair::KeyPair;
    use libp2p_identity::{Keypair, PeerId};
    use tempdir::TempDir;

    use config_utils::modules_dir;
    use fluence_libp2p::RandomPeerId;
    use particle_modules::{AddBlueprint, ModuleRepository};
    use server_config::ServicesConfig;
    use service_modules::load_module;
    use service_modules::Hash;
    use types::peer_scope::PeerScope;
    use workers::{DummyCoreManager, KeyStorage, PeerScopes, Workers};

    use crate::app_services::{ServiceAlias, ServiceType};
    use crate::persistence::load_persisted_services;
    use crate::{ParticleAppServices, ServiceError};

    fn create_pid() -> PeerId {
        let keypair = Keypair::generate_ed25519();

        PeerId::from(keypair.public())
    }

    async fn create_pas(
        root_keypair: Keypair,
        management_pid: PeerId,
        base_dir: PathBuf,
    ) -> ParticleAppServices {
        let persistent_dir = base_dir.join("persistent");
        let ephemeral_dir = base_dir.join("ephemeral");
        let vault_dir = ephemeral_dir.join("..").join("vault");
        let keypairs_dir = persistent_dir.join("..").join("keypairs");
        let workers_dir = persistent_dir.join("..").join("workers");
        let service_memory_limit = server_config::default_service_memory_limit();
        let key_storage = KeyStorage::from_path(keypairs_dir.clone(), root_keypair.clone().into())
            .await
            .expect("Could not load key storage");

        let key_storage = Arc::new(key_storage);

        let root_key_pair: KeyPair = root_keypair.clone().into();

        let core_manager = Arc::new(DummyCoreManager::default().into());

        let scope = PeerScopes::new(
            root_key_pair.get_peer_id(),
            management_pid,
            root_key_pair.get_peer_id(),
            key_storage.clone(),
        );

        let workers = Workers::from_path(workers_dir.clone(), key_storage, core_manager)
            .await
            .expect("Could not load worker registry");

        let workers = Arc::new(workers);

        let config = ServicesConfig::new(
            PeerId::from(root_keypair.public()),
            persistent_dir,
            ephemeral_dir,
            vault_dir,
            HashMap::new(),
            management_pid,
            root_key_pair.get_peer_id(),
            Some(service_memory_limit),
            Default::default(),
        )
        .unwrap();

        let repo = ModuleRepository::new(
            &config.modules_dir,
            &config.blueprint_dir,
            Default::default(),
        );

        ParticleAppServices::new(config, repo, None, None, workers, scope)
    }

    async fn call_add_alias_raw(
        as_manager: bool,
        alias: String,
        service_id: String,
    ) -> Result<(), ServiceError> {
        let base_dir = TempDir::new("test3").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path()).await;

        let client_pid;
        if as_manager {
            client_pid = management_pid;
        } else {
            client_pid = create_pid();
        }

        pas.add_alias(PeerScope::Host, alias, service_id, client_pid)
            .await
    }

    async fn call_add_alias(alias: String, service_id: String) -> Result<(), ServiceError> {
        call_add_alias_raw(true, alias, service_id).await
    }

    async fn create_service(
        pas: &ParticleAppServices,
        module_name: String,
        module: &str,
        peer_scope: PeerScope,
    ) -> Result<String, String> {
        let dep = Hash::from_string(module).unwrap();
        let bp = pas
            .modules
            .add_blueprint(AddBlueprint::new(module_name, vec![dep]))
            .unwrap();

        pas.create_service(peer_scope, ServiceType::Service, bp, RandomPeerId::random())
            .await
            .map_err(|e| e.to_string())
    }

    #[tokio::test]
    async fn test_add_alias_forbidden() {
        let resp = call_add_alias_raw(false, "1".to_string(), "2".to_string()).await;
        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ServiceError::ForbiddenAliasRoot { .. }
        ))
    }

    #[tokio::test]
    async fn test_add_alias_no_service() {
        let resp = call_add_alias("1".to_string(), "2".to_string()).await;
        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ServiceError::NoSuchService(..)
        ));
    }

    #[tokio::test]
    async fn test_get_interface_cache() {
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let base_dir = TempDir::new("test").unwrap();
        let pas = create_pas(root_keypair, management_pid, base_dir.path().into()).await;

        let module = load_module(
            "../crates/nox-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let module_name = "tetra".to_string();
        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: module_name.clone(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };
        let m_hash = pas
            .modules
            .add_module_base64(base64.encode(module), config)
            .unwrap();
        let service_id1 = create_service(&pas, module_name.clone(), &m_hash, PeerScope::Host)
            .await
            .unwrap();
        let service_id2 = create_service(&pas, module_name.clone(), &m_hash, PeerScope::Host)
            .await
            .unwrap();
        let service_id3 = create_service(&pas, module_name, &m_hash, PeerScope::Host)
            .await
            .unwrap();

        let inter1 = pas.get_interface(PeerScope::Host, service_id1, "").unwrap();

        // delete module and check that interfaces will be returned anyway
        let dir = modules_dir(&base_dir.path().to_path_buf().join("persistent"));
        let module_file = dir.join(format!("{m_hash}.wasm"));
        tokio::fs::remove_file(module_file.clone()).await.unwrap();

        let inter2 = pas.get_interface(PeerScope::Host, service_id2, "").unwrap();
        let inter3 = pas.get_interface(PeerScope::Host, service_id3, "").unwrap();

        assert_eq!(module_file.exists(), false);
        assert_eq!(inter1, inter2);
        assert_eq!(inter3, inter2);
    }

    fn upload_tetra_service(pas: &ParticleAppServices, module_name: String) -> String {
        let module = load_module(
            "../crates/nox-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: module_name,
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };
        pas.modules
            .add_module_base64(base64.encode(module), config)
            .unwrap()
    }

    #[tokio::test]
    async fn test_add_alias() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path()).await;

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());
        let service_id1 = create_service(&pas, module_name, &m_hash, PeerScope::Host)
            .await
            .unwrap();

        let alias = "alias";
        let result = pas
            .add_alias(
                PeerScope::Host,
                alias.to_string(),
                service_id1.clone(),
                management_pid,
            )
            .await;
        // result of the add_alias call must be ok
        assert!(result.is_ok(), "{}", result.unwrap_err());

        let (service_1, _) = pas
            .get_service(PeerScope::Host, service_id1.clone(), "")
            .unwrap();
        let service_1_aliases: Vec<ServiceAlias> = service_1.aliases.read().clone();
        // the service's alias list must contain the alias
        assert_eq!(service_1_aliases, vec![alias.to_string()]);

        let (persisted_service_1, _) = load_persisted_services(&pas.config.services_dir)
            .await
            .unwrap()
            .into_iter()
            .find(|(s, _)| s.service_id == service_id1)
            .unwrap();

        // the persisted service's alias list must contain the alias
        assert_eq!(persisted_service_1.aliases, vec![alias.to_string()]);
    }

    #[tokio::test]
    async fn test_add_alias_repeated() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path()).await;

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name.clone(), &m_hash, PeerScope::Host)
            .await
            .unwrap();
        let service_id2 = create_service(&pas, module_name, &m_hash, PeerScope::Host)
            .await
            .unwrap();

        let alias = "alias";
        // add an alias to a service
        pas.add_alias(
            PeerScope::Host,
            alias.to_string(),
            service_id1.clone(),
            management_pid,
        )
        .await
        .unwrap();
        // give the alias to another service
        pas.add_alias(
            PeerScope::Host,
            alias.to_string(),
            service_id2.clone(),
            management_pid,
        )
        .await
        .unwrap();

        let (service_1, _) = pas
            .get_service(PeerScope::Host, service_id1.clone(), "")
            .unwrap();
        let (service_2, _) = pas
            .get_service(PeerScope::Host, service_id2.clone(), "")
            .unwrap();
        let service_aliases_1 = service_1.aliases.read().clone();
        let service_aliases_2 = service_2.aliases.read().clone();
        // the first service's alias list must not contain the alias
        assert_eq!(service_aliases_1, Vec::<String>::new());
        // the second service's alias list must contain the alias
        assert_eq!(service_aliases_2, vec![alias.to_string()]);

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
            .await
            .unwrap()
            .into_iter()
            .collect();
        let (persisted_service_1, _) = persisted_services
            .iter()
            .find(|(service, _)| service.service_id == service_id1)
            .unwrap();
        let (persisted_service_2, _) = persisted_services
            .iter()
            .find(|(service, _)| service.service_id == service_id2)
            .unwrap();
        // the first persisted service's alias list must not contain the alias
        assert_eq!(persisted_service_1.aliases, Vec::<String>::new());
        // the second persisted service's alias list must contain the alias
        assert_eq!(persisted_service_2.aliases, vec![alias.to_string()]);
    }

    #[tokio::test]
    async fn test_add_alias_twice() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path()).await;

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id = create_service(&pas, module_name.clone(), &m_hash, PeerScope::Host)
            .await
            .unwrap();

        let alias = "alias";
        // add an alias to a service
        pas.add_alias(
            PeerScope::Host,
            alias.to_string(),
            service_id.clone(),
            management_pid,
        )
        .await
        .unwrap();
        // give the alias to service again
        pas.add_alias(
            PeerScope::Host,
            alias.to_string(),
            service_id.clone(),
            management_pid,
        )
        .await
        .unwrap();

        let (service, _) = pas
            .get_service(PeerScope::Host, service_id.clone(), "")
            .unwrap();
        let service_aliases = service.aliases.read().clone();
        // the service's alias list must contain only 1 alias
        assert_eq!(service_aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
            .await
            .unwrap()
            .into_iter()
            .collect();
        let (persisted_service, _) = persisted_services
            .iter()
            .find(|(s, _)| s.service_id == service_id)
            .unwrap();

        // the persisted service's alias list must contain only one alias
        assert_eq!(persisted_service.aliases, vec![alias.to_string()]);
    }

    #[tokio::test]
    async fn test_persisted_service() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path()).await;

        let module_name = "tetra".to_string();
        let alias = "alias".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name, &m_hash, PeerScope::Host)
            .await
            .unwrap();
        pas.add_alias(
            PeerScope::Host,
            alias.clone(),
            service_id1.clone(),
            management_pid,
        )
        .await
        .unwrap();
        let (service_1, _) = pas
            .get_service(PeerScope::Host, service_id1.clone(), "")
            .unwrap();
        let service_aliases_1 = service_1.aliases.read().clone();
        assert_eq!(service_aliases_1.len(), 1);
        assert_eq!(service_aliases_1[0], alias);

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
            .await
            .unwrap()
            .into_iter()
            .collect();
        let (persisted_service_1, _) = persisted_services.first().unwrap();
        assert_eq!(service_aliases_1, persisted_service_1.aliases);
        assert_eq!(service_aliases_1, persisted_service_1.aliases);
        assert_eq!(service_1.blueprint_id, persisted_service_1.blueprint_id);
        assert_eq!(service_id1, persisted_service_1.service_id);
        assert_eq!(service_1.owner_id, persisted_service_1.owner_id);
    }

    // TODO: add more tests
    //       - add alias success & fail with service collision & test on rewriting alias
    //       - create_service success & fail
    //       - get_modules success & fail
    //       - get_interface
    //       - list_services
    //       - test on service persisting
}
