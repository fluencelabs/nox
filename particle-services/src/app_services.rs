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
    SecurityTetraplet, ServiceInterface,
};
use humantime_serde::re::humantime::format_duration as pretty;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JValue};

use fluence_libp2p::{peerid_serializer, PeerId};
use health::HealthCheckRegistry;
use key_manager::KeyManager;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_execution::{FunctionOutcome, ParticleParams, ParticleVault};
use particle_modules::ModuleRepository;
use peer_metrics::{
    ServiceCallStats, ServiceMemoryStat, ServiceType as MetricServiceType, ServicesMetrics,
    ServicesMetricsBuiltin,
};
use server_config::ServicesConfig;
use uuid_utils::uuid;

use crate::error::ServiceError;
use crate::error::ServiceError::{AliasAsServiceId, Forbidden, NoSuchAlias};
use crate::health::PersistedServiceHealth;
use crate::persistence::{
    load_persisted_services, persist_service, remove_persisted_service, PersistedService,
};
use crate::ServiceError::{
    ForbiddenAlias, ForbiddenAliasRoot, ForbiddenAliasWorker, InternalError, NoSuchService,
};

type ServiceId = String;
type ServiceAlias = String;
type Services = HashMap<ServiceId, Service>;
type Aliases = HashMap<PeerId, HashMap<ServiceAlias, ServiceId>>;

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

#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub id: String,
    pub blueprint_id: String,
    pub service_type: ServiceType,
    #[serde(with = "peerid_serializer")]
    pub owner_id: PeerId,
    pub aliases: Vec<ServiceAlias>,
    #[serde(with = "peerid_serializer")]
    pub worker_id: PeerId,
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
    pub aliases: Vec<ServiceAlias>,
    pub worker_id: PeerId,
}

impl Service {
    pub fn new(
        service: Mutex<AppService>,
        service_id: String,
        blueprint_id: String,
        service_type: ServiceType,
        owner_id: PeerId,
        aliases: Vec<ServiceAlias>,
        worker_id: PeerId,
    ) -> Self {
        Self {
            service,
            service_id,
            blueprint_id,
            service_type,
            owner_id,
            aliases,
            worker_id,
        }
    }

    pub fn persist(&self, services_dir: &Path) -> Result<(), ServiceError> {
        persist_service(services_dir, PersistedService::from_service(self))
    }

    pub fn remove_alias(&mut self, alias: &str) {
        if let Some(pos) = self.aliases.iter().position(|x| *x == alias) {
            self.aliases.remove(pos);
        }
    }

    pub fn add_alias(&mut self, alias: String) {
        self.aliases.push(alias);
    }

    pub fn get_info(&self, id: &str) -> ServiceInfo {
        ServiceInfo {
            id: id.to_string(),
            blueprint_id: self.blueprint_id.clone(),
            service_type: self.service_type.clone(),
            owner_id: self.owner_id,
            aliases: self.aliases.clone(),
            worker_id: self.worker_id,
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
#[derivative(Debug, Clone)]
pub struct ParticleAppServices {
    config: ServicesConfig,
    // TODO: move vault to Plumber or Actor
    pub vault: ParticleVault,
    services: Arc<RwLock<Services>>,
    modules: ModuleRepository,
    aliases: Arc<RwLock<Aliases>>,
    #[derivative(Debug = "ignore")]
    key_manager: KeyManager,
    pub metrics: Option<ServicesMetrics>,
    health: Option<PersistedServiceHealth>,
}

/// firstly, try to find by alias in worker scope, secondly, in root scope
pub fn resolve_alias(
    particle_id: &str,
    alias: String,
    aliases: &Aliases,
    worker_id: PeerId,
    local_peer_id: PeerId,
) -> Option<String> {
    if alias == "spell" || alias == "self" {
        if let Some(spell_id) = ParticleParams::get_spell_id(particle_id) {
            return Some(spell_id);
        }
    }

    let worker_scope: Option<String> = try { aliases.get(&worker_id)?.get(&alias).cloned()? };

    if let Some(result) = worker_scope {
        Some(result)
    } else {
        // try to find in root scope if not found
        aliases.get(&local_peer_id)?.get(&alias).cloned()
    }
}

pub fn get_service<'l>(
    particle_id: &str,
    services: &'l Services,
    aliases: &Aliases,
    worker_id: PeerId,
    local_peer_id: PeerId,
    id_or_alias: String,
) -> Result<(&'l Service, String), ServiceError> {
    // retrieve service by service id
    if let Some(service) = services.get(&id_or_alias) {
        return Ok((service, id_or_alias));
    }

    // retrieve service by alias
    let by_alias: Option<_> = try {
        let resolved_id = resolve_alias(
            particle_id,
            id_or_alias.clone(),
            aliases,
            worker_id,
            local_peer_id,
        )?;
        let service = services.get(&resolved_id)?;
        (service, resolved_id.clone())
    };

    by_alias.ok_or(NoSuchService(id_or_alias))
}

fn get_service_mut<'l>(
    services: &'l mut Services,
    worker_id: PeerId,
    service_id: &str,
) -> Result<&'l mut Service, ServiceError> {
    let service = services
        .get_mut(service_id)
        .ok_or(NoSuchService(service_id.to_string()))?;

    if service.worker_id != worker_id {
        // service is deployed on another worker_id
        Err(ServiceError::AliasWrongWorkerId {
            service_id: service_id.to_string(),
            worker_id: service.worker_id,
        })
    } else {
        Ok(service)
    }
}

impl ParticleAppServices {
    pub fn new(
        config: ServicesConfig,
        modules: ModuleRepository,
        metrics: Option<ServicesMetrics>,
        health_registry: Option<&mut HealthCheckRegistry>,
        key_manager: KeyManager,
    ) -> Self {
        let vault = ParticleVault::new(config.particles_vault_dir.clone());

        let health = health_registry.map(|registry| {
            let persisted_services = PersistedServiceHealth::new();
            registry.register("persisted_services", persisted_services.clone());
            persisted_services
        });
        let mut this = Self {
            config,
            vault,
            services: <_>::default(),
            modules,
            aliases: <_>::default(),
            key_manager,
            metrics,
            health,
        };

        this.create_persisted_services();

        this
    }

    pub fn create_service(
        &self,
        service_type: ServiceType,
        blueprint_id: String,
        owner_id: PeerId,
        worker_id: PeerId,
    ) -> Result<String, ServiceError> {
        let service_id = uuid::Uuid::new_v4().to_string();
        self.create_service_inner(
            service_type,
            blueprint_id,
            owner_id,
            worker_id,
            service_id.clone(),
            vec![],
        )?;
        Ok(service_id)
    }

    pub fn service_exists(&self, service_id: &str) -> bool {
        self.services.read().get(service_id).is_some()
    }

    pub fn get_service_info(
        &self,
        particle_id: &str,
        worker_id: PeerId,
        service_id_or_alias: String,
    ) -> Result<ServiceInfo, ServiceError> {
        let services_read = self.services.read();
        let (service, service_id) = get_service(
            particle_id,
            &services_read,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            service_id_or_alias,
        )?;

        Ok(service.get_info(&service_id))
    }

    pub fn remove_services(&self, worker_id: PeerId) -> Result<(), ServiceError> {
        let removed_services: Vec<ServiceInfo> = {
            // TODO: it's highly ineffective, services should be organised by workers
            self.services
                .write()
                .extract_if(|_, srv| srv.worker_id == worker_id)
                .map(|(id, srv)| srv.get_info(&id))
                .collect()
        };

        for srv in removed_services {
            if let Err(err) = remove_persisted_service(&self.config.services_dir, srv.id.clone()) {
                log::warn!(
                    "Error while removing persisted service for {}: {:?}",
                    srv.id,
                    err
                )
            }

            if let Some(aliases) = self.aliases.write().get_mut(&srv.worker_id) {
                for alias in srv.aliases {
                    aliases.remove(&alias);
                }
            }
        }

        Ok(())
    }

    pub fn remove_service(
        &self,
        particle_id: &str,
        worker_id: PeerId,
        service_id_or_alias: &str,
        init_peer_id: PeerId,
        allow_remove_spell: bool,
    ) -> Result<(), ServiceError> {
        let removal_start_time = Instant::now();
        let service_id = {
            let services_read = self.services.read();
            let (service, service_id) = get_service(
                particle_id,
                &services_read,
                &self.aliases.read(),
                worker_id,
                self.config.local_peer_id,
                service_id_or_alias.to_string(),
            )?;

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
            //  builtins_management_peer_id is a HACKity hack:
            //      It actually needs to be able to remove only builtins (services deployed from FS on start),
            //      but there's no way to tell which one's are "builtins", so we allow it to remove
            //      all services.
            if service.worker_id != init_peer_id
                && service.owner_id != init_peer_id
                && !self.key_manager.is_management(init_peer_id)
            {
                return Err(Forbidden {
                    user: init_peer_id,
                    function: "remove_service",
                    reason: "only creator can remove service",
                });
            }

            service_id
        };

        if let Err(err) = remove_persisted_service(&self.config.services_dir, service_id.clone()) {
            log::warn!(
                "Error while removing persisted service for {}: {:?}",
                service_id,
                err
            )
        }
        let service = self.services.write().remove(&service_id).unwrap();
        let service_type = self.get_service_type(&service, &service.worker_id);

        if let Some(aliases) = self.aliases.write().get_mut(&service.worker_id) {
            for alias in service.aliases.iter() {
                aliases.remove(alias);
            }
        }

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
        let services = self.services.read();
        let aliases = self.aliases.read();
        let worker_id = particle.host_id;
        let timestamp = particle.timestamp;

        let service = get_service(
            &particle.id,
            &services,
            &aliases,
            worker_id,
            self.config.local_peer_id,
            function_args.service_id.clone(),
        );

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
        let service_type = self.get_service_type(service, &worker_id);

        // TODO: move particle vault creation to aquamarine::particle_functions
        if create_vault {
            self.vault.create(&particle.id)?;
        }

        let params = CallParameters {
            host_id: worker_id.to_string(),
            particle_id: particle.id,
            init_peer_id: particle.init_peer_id.to_string(),
            tetraplets: function_args
                .tetraplets
                .into_iter()
                .map(|sts| {
                    sts.into_iter()
                        .map(|st| SecurityTetraplet {
                            peer_pk: st.peer_pk,
                            service_id: st.service_id,
                            function_name: st.function_name,
                            json_path: st.json_path,
                        })
                        .collect()
                })
                .collect(),
            service_id: service_id.clone(),
            service_creator_peer_id: service.owner_id.to_string(),
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

    #[allow(clippy::too_many_arguments)]
    pub fn call_function(
        &self,
        worker_id: PeerId,
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
            host_id: worker_id,
            timestamp: now_ms() as u64,
            ttl: particle_ttl.as_millis() as u32,
            script: "".to_string(),
            signature: vec![],
        };

        self.call_service(args, particle, false)
    }

    fn add_alias_inner(
        &self,
        alias: String,
        worker_id: PeerId,
        service_id: ServiceId,
    ) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = get_service_mut(&mut services, worker_id, &service_id)?;
        service.add_alias(alias);
        service.persist(&self.config.services_dir)
    }

    fn get_service_id(&self, worker_id: PeerId, alias: &str) -> Option<ServiceId> {
        self.aliases
            .read()
            .get(&worker_id)
            .and_then(|aliases| aliases.get(alias))
            .cloned()
    }

    fn remove_alias(
        &self,
        alias: String,
        worker_id: PeerId,
        service_id: &str,
    ) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = get_service_mut(&mut services, worker_id, service_id)?;
        service.remove_alias(&alias);
        service.persist(&self.config.services_dir)
    }

    pub fn add_alias(
        &self,
        alias: String,
        worker_id: PeerId,
        service_id: String,
        init_peer_id: PeerId,
    ) -> Result<(), ServiceError> {
        let is_management = self.key_manager.is_management(init_peer_id);
        let is_root_scope = self.key_manager.is_host(worker_id);
        let is_worker = init_peer_id == worker_id;
        let worker_creator = self
            .key_manager
            .get_worker_creator(worker_id)
            .map_err(|e| InternalError(format!("{e:?}")))?; // worker creator is always set
        let is_worker_creator = init_peer_id == worker_creator;

        if is_root_scope && !is_management {
            return Err(ForbiddenAliasRoot(init_peer_id));
        } else if !is_worker && !is_management && !is_worker_creator {
            return Err(ForbiddenAliasWorker(init_peer_id));
        }

        // alias can't be equal to any existent service id
        if self.service_exists(&alias) {
            return Err(AliasAsServiceId(alias));
        }

        if alias == "spell" || alias == "self" {
            return Err(ForbiddenAlias(alias));
        }

        if !self.service_exists(&service_id) {
            return Err(NoSuchService(service_id));
        }

        let prev_srv_id = self.get_service_id(worker_id, &alias);
        if let Some(srv_id) = prev_srv_id {
            self.remove_alias(alias.clone(), worker_id, &srv_id)?;
        }

        self.add_alias_inner(alias.clone(), worker_id, service_id.clone())?;
        self.aliases
            .write()
            .entry(worker_id)
            .or_default()
            .insert(alias, service_id);

        Ok(())
    }

    pub fn resolve_alias(
        &self,
        particle_id: &str,
        worker_id: PeerId,
        alias: String,
    ) -> Result<String, ServiceError> {
        let aliases = self.aliases.read();
        resolve_alias(
            particle_id,
            alias.clone(),
            &aliases,
            worker_id,
            self.config.local_peer_id,
        )
        .ok_or_else(|| NoSuchAlias(alias, worker_id))
    }

    pub fn to_service_id(
        &self,
        particle_id: &str,
        worker_id: PeerId,
        service_id_or_alias: String,
    ) -> Result<String, ServiceError> {
        let services = self.services.read();
        let (_, service_id) = get_service(
            particle_id,
            &services,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            service_id_or_alias,
        )?;
        Ok(service_id)
    }

    pub fn get_service_owner(
        &self,
        particle_id: &str,
        id_or_alias: String,
        worker_id: PeerId,
    ) -> Result<PeerId, ServiceError> {
        let services = self.services.read();
        let (service, _) = get_service(
            particle_id,
            &services,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            id_or_alias,
        )?;

        Ok(service.owner_id)
    }

    pub fn check_service_worker_id(
        &self,
        particle_id: &str,
        id_or_alias: String,
        worker_id: PeerId,
    ) -> Result<(), ServiceError> {
        let services = self.services.read();
        let (service, _) = get_service(
            particle_id,
            &services,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            id_or_alias.clone(),
        )?;

        if service.worker_id != worker_id {
            Err(ServiceError::CallServiceFailedWrongWorker {
                service_id: id_or_alias,
                worker_id,
            })
        } else {
            Ok(())
        }
    }

    pub fn get_interface(
        &self,
        particle_id: &str,
        service_id: String,
        worker_id: PeerId,
    ) -> Result<JValue, ServiceError> {
        let services = self.services.read();
        let (service, _) = get_service(
            particle_id,
            &services,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            service_id,
        )?;

        Ok(self.modules.get_facade_interface(&service.blueprint_id)?)
    }

    pub fn list_services_with_info(&self) -> Vec<ServiceInfo> {
        let services = self.services.read();
        services
            .iter()
            .map(|(id, service)| service.get_info(id))
            .collect()
    }

    pub fn list_services(&self, worker_id: PeerId) -> Vec<JValue> {
        let services = self.services.read();
        let services = services
            .iter()
            .filter(|(_, srv)| srv.worker_id.eq(&worker_id))
            .map(|(id, srv)| json!(srv.get_info(id)))
            .collect();

        services
    }

    pub fn get_service_mem_stats(
        &self,
        particle_id: &str,
        worker_id: PeerId,
        service_id: String,
    ) -> Result<Vec<JValue>, JError> {
        let services = self.services.read();
        let (service, _) = get_service(
            particle_id,
            &services,
            &self.aliases.read(),
            worker_id,
            self.config.local_peer_id,
            service_id,
        )?;

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

    fn create_persisted_services(&mut self) {
        let services =
            load_persisted_services(&self.config.services_dir, self.config.local_peer_id);
        let loaded_service_count = services.len();
        if let Some(h) = self.health.as_mut() {
            h.start_creation()
        }
        let services = services.into_iter();

        let services = services.filter_map(|r| match r {
            Ok(service) => service.into(),
            Err(err) => {
                log::warn!("Error loading one of persisted services: {:?}", err);
                None
            }
        });

        let mut created_service_count = 0;
        for s in services {
            let worker_id = s.worker_id.expect("every service must have worker id");
            let start = Instant::now();
            // If the service_type doesn't set in PersitedService, will try to find out if it's a spell by blueprint name
            // This is mostly done for migration from the old detection method to the new.
            let service_type = s.service_type.unwrap_or_else(|| {
                let is_spell: Option<_> = try {
                    let blueprint_name = self
                        .modules
                        .get_blueprint_from_cache(&s.blueprint_id)
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
            let result = self.create_service_inner(
                service_type,
                s.blueprint_id,
                s.owner_id,
                worker_id,
                s.service_id.clone(),
                s.aliases.clone(),
            );
            let replaced = match result {
                Ok(replaced) => replaced,
                Err(err) => {
                    #[rustfmt::skip]
                    log::warn!("Error creating service for persisted service {}: {:#?}", s.service_id, err);
                    continue;
                }
            };

            let mut binding = self.aliases.write();
            let aliases = binding.entry(worker_id).or_default();
            for alias in s.aliases.iter() {
                let old = aliases.insert(alias.clone(), s.service_id.clone());
                if let Some(old) = old {
                    log::warn!(
                        "Alias `{}` is the same for {} and {}",
                        alias,
                        old,
                        s.service_id
                    );
                }
            }

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );
            created_service_count += 1;
            log::info!(
                "Persisted service {} created in {}, aliases: {:?}",
                s.service_id,
                pretty(start.elapsed()),
                s.aliases
            );
        }
        if created_service_count == loaded_service_count {
            if let Some(h) = self.health.as_mut() {
                h.finish_creation()
            }
        }
    }

    fn create_service_inner(
        &self,
        service_type: ServiceType,
        blueprint_id: String,
        owner_id: PeerId,
        worker_id: PeerId,
        service_id: String,
        aliases: Vec<String>,
    ) -> Result<Option<Service>, ServiceError> {
        let creation_start_time = Instant::now();
        let service = self
            .create_app_service(blueprint_id.clone(), service_id.clone())
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
            worker_id,
        );
        // Save created service to disk, so it is recreated on restart
        service.persist(&self.config.services_dir)?;
        let service_type = self.get_service_type(&service, &worker_id);
        let replaced = self.services.write().insert(service_id.clone(), service);
        if let Some(m) = self.metrics.as_ref() {
            let creation_end_time = creation_start_time.elapsed().as_secs();
            m.observe_created(service_id, service_type, stats, creation_end_time as f64);
        }

        Ok(replaced)
    }

    fn create_app_service(
        &self,
        blueprint_id: String,
        service_id: String,
    ) -> Result<AppService, ServiceError> {
        let mut modules_config = self.modules.resolve_blueprint(&blueprint_id)?;
        modules_config
            .iter_mut()
            .for_each(|module| self.vault.inject_vault(module));

        // TODO: I guess this part is obsolete. Need to record the service memory limit instead.
        //if let Some(metrics) = self.metrics.as_ref() {
        //     metrics.observe_service_config(self.config.default_service_memory_limit.as_u64());
        //}

        let app_config = AppServiceConfig {
            service_working_dir: self.config.workdir.join(&service_id),
            service_base_dir: self.config.workdir.clone(),
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

        log::debug!(
            "Creating service {}, envs: {:?}",
            service_id,
            self.config.envs
        );

        AppService::new(app_config, service_id, self.config.envs.clone())
            .map_err(ServiceError::Engine)
    }

    fn get_service_type(&self, service: &Service, worker_id: &PeerId) -> MetricServiceType {
        let allowed_alias = if self.config.local_peer_id.eq(worker_id) {
            service.aliases.first().cloned()
        } else if service
            .aliases
            .first()
            .map(|alias| alias == "worker-spell")
            .unwrap_or(false)
        {
            Some("worker-spell".to_string())
        } else {
            None
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
    use std::fs::remove_file;
    use std::path::PathBuf;

    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_app_service::{TomlMarineModuleConfig, TomlMarineNamedModuleConfig};
    use libp2p_identity::{Keypair, PeerId};
    use tempdir::TempDir;

    use config_utils::{modules_dir, to_peer_id};
    use fluence_libp2p::RandomPeerId;
    use key_manager::KeyManager;
    use particle_modules::{AddBlueprint, ModuleRepository};
    use server_config::ServicesConfig;
    use service_modules::load_module;
    use service_modules::Hash;

    use crate::app_services::ServiceType;
    use crate::persistence::load_persisted_services;
    use crate::{ParticleAppServices, ServiceError};

    fn create_pid() -> PeerId {
        let keypair = Keypair::generate_ed25519();

        PeerId::from(keypair.public())
    }

    fn create_pas(
        root_keypair: Keypair,
        management_pid: PeerId,
        base_dir: PathBuf,
    ) -> ParticleAppServices {
        let startup_kp = Keypair::generate_ed25519();
        let vault_dir = base_dir.join("..").join("vault");
        let keypairs_dir = base_dir.join("..").join("keypairs");
        let workers_dir = base_dir.join("..").join("workers");
        let service_memory_limit = server_config::default_service_memory_limit();
        let key_manager = KeyManager::new(
            keypairs_dir,
            workers_dir,
            root_keypair.clone().into(),
            management_pid,
            to_peer_id(&startup_kp),
        );

        let config = ServicesConfig::new(
            PeerId::from(root_keypair.public()),
            base_dir,
            vault_dir,
            HashMap::new(),
            management_pid,
            to_peer_id(&startup_kp),
            Some(service_memory_limit),
            Default::default(),
        )
        .unwrap();

        let repo = ModuleRepository::new(
            &config.modules_dir,
            &config.blueprint_dir,
            &config.particles_vault_dir,
            Default::default(),
        );

        ParticleAppServices::new(config, repo, None, None, key_manager)
    }

    fn call_add_alias_raw(
        as_manager: bool,
        alias: String,
        service_id: String,
    ) -> Result<(), ServiceError> {
        let base_dir = TempDir::new("test3").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path());

        let client_pid;
        if as_manager {
            client_pid = management_pid;
        } else {
            client_pid = create_pid();
        }

        pas.add_alias(alias, local_pid, service_id, client_pid)
    }

    fn call_add_alias(alias: String, service_id: String) -> Result<(), ServiceError> {
        call_add_alias_raw(true, alias, service_id)
    }

    fn create_service(
        pas: &ParticleAppServices,
        module_name: String,
        module: &str,
        worker_id: PeerId,
    ) -> Result<String, String> {
        let dep = Hash::from_string(module).unwrap();
        let bp = pas
            .modules
            .add_blueprint(AddBlueprint::new(module_name, vec![dep]))
            .unwrap();

        pas.create_service(ServiceType::Service, bp, RandomPeerId::random(), worker_id)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn test_add_alias_forbidden() {
        let resp = call_add_alias_raw(false, "1".to_string(), "2".to_string());
        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ServiceError::ForbiddenAliasRoot { .. }
        ))
    }

    #[test]
    fn test_add_alias_no_service() {
        let resp = call_add_alias("1".to_string(), "2".to_string());
        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ServiceError::NoSuchService(..)
        ));
    }

    #[test]
    fn test_get_interface_cache() {
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let base_dir = TempDir::new("test").unwrap();
        let pas = create_pas(root_keypair, management_pid, base_dir.path().into());

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
        let service_id1 = create_service(&pas, module_name.clone(), &m_hash, local_pid).unwrap();
        let service_id2 = create_service(&pas, module_name.clone(), &m_hash, local_pid).unwrap();
        let service_id3 = create_service(&pas, module_name, &m_hash, local_pid).unwrap();

        let inter1 = pas.get_interface("", service_id1, local_pid).unwrap();

        // delete module and check that interfaces will be returned anyway
        let dir = modules_dir(base_dir.path());
        let module_file = dir.join(format!("{m_hash}.wasm"));
        remove_file(module_file.clone()).unwrap();

        let inter2 = pas.get_interface("", service_id2, local_pid).unwrap();
        let inter3 = pas.get_interface("", service_id3, local_pid).unwrap();

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

    #[test]
    fn test_add_alias() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());
        let service_id1 = create_service(&pas, module_name, &m_hash, local_pid).unwrap();

        let alias = "alias";
        let result = pas.add_alias(
            alias.to_string(),
            local_pid,
            service_id1.clone(),
            management_pid,
        );
        // result of the add_alias call must be ok
        assert!(result.is_ok(), "{}", result.unwrap_err());

        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();
        // the service's alias list must contain the alias
        assert_eq!(service_1.aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> =
            load_persisted_services(&pas.config.services_dir, local_pid)
                .into_iter()
                .collect::<Result<_, _>>()
                .unwrap();
        let persisted_service_1 = persisted_services
            .iter()
            .find(|s| s.service_id == service_id1)
            .unwrap();

        // the persisted service's alias list must contain the alias
        assert_eq!(persisted_service_1.aliases, vec![alias.to_string()]);
    }

    #[test]
    fn test_add_alias_repeated() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name.clone(), &m_hash, local_pid).unwrap();
        let service_id2 = create_service(&pas, module_name, &m_hash, local_pid).unwrap();

        let alias = "alias";
        // add an alias to a service
        pas.add_alias(
            alias.to_string(),
            local_pid,
            service_id1.clone(),
            management_pid,
        )
        .unwrap();
        // give the alias to another service
        pas.add_alias(
            alias.to_string(),
            local_pid,
            service_id2.clone(),
            management_pid,
        )
        .unwrap();

        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();
        let service_2 = services.get(&service_id2).unwrap();
        // the first service's alias list must not contain the alias
        assert_eq!(service_1.aliases, Vec::<String>::new());
        // the second service's alias list must contain the alias
        assert_eq!(service_2.aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> =
            load_persisted_services(&pas.config.services_dir, local_pid)
                .into_iter()
                .collect::<Result<_, _>>()
                .unwrap();
        let persisted_service_1 = persisted_services
            .iter()
            .find(|s| s.service_id == service_id1)
            .unwrap();
        let persisted_service_2 = persisted_services
            .iter()
            .find(|s| s.service_id == service_id2)
            .unwrap();
        // the first persisted service's alias list must not contain the alias
        assert_eq!(persisted_service_1.aliases, Vec::<String>::new());
        // the second persisted service's alias list must contain the alias
        assert_eq!(persisted_service_2.aliases, vec![alias.to_string()]);
    }

    #[test]
    fn test_add_alias_twice() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id = create_service(&pas, module_name.clone(), &m_hash, local_pid).unwrap();

        let alias = "alias";
        // add an alias to a service
        pas.add_alias(
            alias.to_string(),
            local_pid,
            service_id.clone(),
            management_pid,
        )
        .unwrap();
        // give the alias to service again
        pas.add_alias(
            alias.to_string(),
            local_pid,
            service_id.clone(),
            management_pid,
        )
        .unwrap();

        let services = pas.services.read();
        let service = services.get(&service_id).unwrap();

        // the service's alias list must contain only 1 alias
        assert_eq!(service.aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> =
            load_persisted_services(&pas.config.services_dir, local_pid)
                .into_iter()
                .collect::<Result<_, _>>()
                .unwrap();
        let persisted_service = persisted_services
            .iter()
            .find(|s| s.service_id == service_id)
            .unwrap();

        // the persisted service's alias list must contain only one alias
        assert_eq!(persisted_service.aliases, vec![alias.to_string()]);
    }

    #[test]
    fn test_persisted_service() {
        let base_dir = TempDir::new("test4").unwrap();
        let root_keypair = Keypair::generate_ed25519();
        let local_pid = PeerId::from(root_keypair.public());
        let management_pid = create_pid();
        let pas = create_pas(root_keypair, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let alias = "alias".to_string();
        let m_hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name, &m_hash, local_pid).unwrap();
        pas.add_alias(
            alias.clone(),
            local_pid,
            service_id1.clone(),
            management_pid,
        )
        .unwrap();
        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();
        assert_eq!(service_1.aliases.len(), 1);
        assert_eq!(service_1.aliases[0], alias);

        let persisted_services: Vec<_> =
            load_persisted_services(&pas.config.services_dir, local_pid)
                .into_iter()
                .collect::<Result<_, _>>()
                .unwrap();
        let persisted_service_1 = persisted_services.first().unwrap();
        assert_eq!(service_1.aliases, persisted_service_1.aliases);
        assert_eq!(service_1.aliases, persisted_service_1.aliases);
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
