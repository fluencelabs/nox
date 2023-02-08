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
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};

use derivative::Derivative;
use fluence_app_service::{
    AppService, AppServiceError, CallParameters, DefaultWasmBackend, MarineError,
    SecurityTetraplet, ServiceInterface,
};
use humantime_serde::re::humantime::format_duration as pretty;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Value as JValue};

use fluence_libp2p::PeerId;
use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_execution::{FunctionOutcome, ParticleParams, ParticleVault, VaultError};
use particle_modules::ModuleRepository;
use peer_metrics::{
    ServiceCallStats, ServiceMemoryStat, ServiceType, ServicesMetrics, ServicesMetricsBuiltin,
};
use server_config::ServicesConfig;
use uuid_utils::uuid;

use crate::app_service::create_app_service;
use crate::error::ServiceError;
use crate::error::ServiceError::{AliasAsServiceId, Forbidden, NoSuchAlias};
use crate::persistence::{
    load_persisted_services, persist_service, remove_persisted_service, PersistedService,
};

type Services = Arc<RwLock<HashMap<String, Service>>>;
type Aliases = Arc<RwLock<HashMap<String, String>>>;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Service {
    #[derivative(Debug(format_with = "fmt_service"))]
    pub service: Mutex<AppService<DefaultWasmBackend>>,
    pub blueprint_id: String,
    pub owner_id: PeerId,
    pub aliases: Vec<String>,
}

impl Service {
    pub fn remove_alias(&mut self, alias: &str) {
        self.aliases.retain(|a| a.ne(alias));
    }

    pub fn add_alias(&mut self, alias: String) {
        self.aliases.push(alias);
    }
}

impl Deref for Service {
    type Target = Mutex<AppService<DefaultWasmBackend>>;

    fn deref(&self) -> &Self::Target {
        &self.service
    }
}

fn fmt_service(
    _: &Mutex<AppService<DefaultWasmBackend>>,
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

#[derive(Debug, Clone)]
pub struct ParticleAppServices {
    config: ServicesConfig,
    // TODO: move vault to Plumber or Actor
    vault: ParticleVault,
    services: Services,
    modules: ModuleRepository,
    aliases: Aliases,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
    pub metrics: Option<ServicesMetrics>,
}

pub fn get_service<'l>(
    services: &'l HashMap<String, Service>,
    aliases: &HashMap<String, String>,
    id_or_alias: String,
) -> Result<(&'l Service, String), String> {
    // retrieve service by service id
    if let Some(service) = services.get(&id_or_alias) {
        return Ok((service, id_or_alias));
    }

    // retrieve service by alias
    let by_alias: Option<_> = try {
        let resolved_id = aliases.get(&id_or_alias)?;
        let service = services.get(resolved_id)?;
        (service, resolved_id.clone())
    };

    by_alias.ok_or(id_or_alias)
}

impl ParticleAppServices {
    pub fn new(
        config: ServicesConfig,
        modules: ModuleRepository,
        metrics: Option<ServicesMetrics>,
    ) -> Self {
        let vault = ParticleVault::new(config.particles_vault_dir.clone());
        let management_peer_id = config.management_peer_id;
        let builtins_management_peer_id = config.builtins_management_peer_id;
        let this = Self {
            config,
            vault,
            services: <_>::default(),
            modules,
            aliases: <_>::default(),
            management_peer_id,
            builtins_management_peer_id,
            metrics,
        };

        this.create_persisted_services();

        this
    }

    pub fn create_service(
        &self,
        blueprint_id: String,
        init_peer_id: PeerId,
    ) -> Result<String, ServiceError> {
        let service_id = uuid::Uuid::new_v4().to_string();
        self.create_service_inner(blueprint_id, init_peer_id, service_id.clone(), vec![])?;
        Ok(service_id)
    }

    pub fn remove_service(
        &self,
        service_id_or_alias: String,
        init_peer_id: PeerId,
        allow_remove_spell: bool,
    ) -> Result<(), ServiceError> {
        let removal_start_time = Instant::now();
        let service_id = {
            let services_read = self.services.read();
            let (service, service_id) =
                get_service(&services_read, &self.aliases.read(), service_id_or_alias)
                    .map_err(ServiceError::NoSuchService)?;

            // tmp hack to forbid spell removal via srv.remove
            let blueprint_name = self
                .modules
                .get_blueprint_from_cache(&service.blueprint_id)?
                .name;
            if blueprint_name == "spell" && !allow_remove_spell {
                return Err(Forbidden {
                    user: init_peer_id,
                    function: "remove_service",
                    reason: "cannot remove a spell",
                });
            } else if blueprint_name != "spell" && allow_remove_spell {
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
            //  builtins_management_peer_id is a HACKity hack:
            //      It actually needs to be able to remove only builtins (services deployed from FS on start),
            //      but there's no way to tell which one's are "builtins", so we allow it to remove
            //      all services.
            if service.owner_id != init_peer_id
                && self.management_peer_id != init_peer_id
                && self.builtins_management_peer_id != init_peer_id
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
        let mut aliases = self.aliases.write();
        for alias in service.aliases.iter() {
            aliases.remove(alias);
        }

        let removal_end_time = removal_start_time.elapsed().as_secs();
        if let Some(metrics) = self.metrics.as_ref() {
            metrics.observe_removed(removal_end_time as f64);
        }

        Ok(())
    }

    pub fn call_service(
        &self,
        mut function_args: Args,
        particle: ParticleParams,
    ) -> FunctionOutcome {
        let call_time_start = Instant::now();
        let services = self.services.read();
        let aliases = self.aliases.read();
        let host_id = self.config.local_peer_id.to_string();
        let timestamp = particle.timestamp;

        let service = get_service(&services, &aliases, function_args.service_id);
        let (service, service_id) = match service {
            Ok(found) => found,
            // If service is not found, report it
            Err(service_id) => {
                // move field back
                function_args.service_id = service_id;
                return FunctionOutcome::NotDefined {
                    args: function_args,
                    params: particle,
                };
            }
        };

        let service_type = ServiceType::Service(service.aliases.first().cloned());

        // TODO: move particle vault creation to aquamarine::particle_functions
        self.create_vault(&particle.id)?;
        let params = CallParameters {
            host_id,
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

        let mut service = service.lock();
        let old_memory = service.module_memory_stats();
        let old_mem_usage = ServicesMetricsBuiltin::get_used_memory(&old_memory);
        // TODO: set execution timeout https://github.com/fluencelabs/fluence/issues/1212
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

        let call_time_sec = call_time_start.elapsed().as_secs_f64();
        let new_memory = service.module_memory_stats();
        let new_memory_usage = ServicesMetricsBuiltin::get_used_memory(&new_memory);

        let memory_delta_bytes = new_memory_usage - old_mem_usage;
        let stats = ServiceCallStats::Success {
            memory_delta_bytes: memory_delta_bytes as f64,
            call_time_sec,
            timestamp,
        };

        if let Some(metrics) = self.metrics.as_ref() {
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

    pub fn call_function(
        &self,
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
            timestamp: now_ms() as u64,
            ttl: particle_ttl.as_millis() as u32,
            script: "".to_string(),
            signature: vec![],
        };

        self.call_service(args, particle)
    }

    pub fn add_alias(
        &self,
        alias: String,
        service_id: String,
        init_peer_id: PeerId,
    ) -> Result<(), ServiceError> {
        if init_peer_id != self.management_peer_id
            && init_peer_id != self.builtins_management_peer_id
        {
            return Err(Forbidden {
                user: init_peer_id,
                function: "add_alias",
                reason: "only management peer id can add aliases",
            });
        };

        // if a client trying to add an alias that equals some created service id
        // return an error
        if self.services.read().get(&alias).is_some() {
            return Err(AliasAsServiceId(alias));
        }

        let mut services = self.services.write();

        let service = services
            .get_mut(&service_id)
            .ok_or_else(|| ServiceError::NoSuchService(service_id.clone()))?;
        service.add_alias(alias.clone());
        let persisted_new = PersistedService::from_service(service_id.clone(), service);

        // Find a service with the same alias if any
        let previous_owner_id = {
            let lock = self.aliases.read();
            lock.get(&alias).cloned()
        };

        // If there is such a service remove the alias from its list of aliases
        let previous_owner = try {
            let previous_owner_id = previous_owner_id?;
            let previous_owner_service = services.get_mut(&previous_owner_id)?;
            previous_owner_service.remove_alias(&alias);
            PersistedService::from_service(previous_owner_id, previous_owner_service)
        };

        drop(services);
        if let Some(previous_owner) = previous_owner {
            // Save the updated aliases list of the previous owner of the alias on disk
            persist_service(&self.config.services_dir, previous_owner)?;
        }
        // Save the target service with the new alias on disk
        persist_service(&self.config.services_dir, persisted_new)?;

        self.aliases.write().insert(alias, service_id.clone());

        Ok(())
    }

    pub fn resolve_alias(&self, alias: String) -> Result<String, ServiceError> {
        let aliases = self.aliases.read();
        let service_id = aliases.get(&alias);

        service_id.cloned().ok_or(NoSuchAlias(alias))
    }

    pub fn to_service_id(&self, service_id_or_alias: String) -> Result<String, ServiceError> {
        let services = self.services.read();
        let (_, service_id) = get_service(&services, &self.aliases.read(), service_id_or_alias)
            .map_err(ServiceError::NoSuchService)?;
        Ok(service_id)
    }

    pub fn get_service_owner(&self, service_id: String) -> Result<PeerId, ServiceError> {
        let services_read = self.services.read();
        get_service(&services_read, &self.aliases.read(), service_id)
            .map_err(ServiceError::NoSuchService)
            .map(|(srv, _)| srv.owner_id)
    }

    pub fn get_interface(&self, service_id: String) -> Result<JValue, ServiceError> {
        let services = self.services.read();
        let (service, _) = get_service(&services, &self.aliases.read(), service_id)
            .map_err(ServiceError::NoSuchService)?;

        Ok(self.modules.get_facade_interface(&service.blueprint_id)?)
    }

    pub fn list_services_with_blueprints(&self) -> Vec<(String, String)> {
        let services = self.services.read();
        services
            .iter()
            .map(|(id, service)| (id.clone(), service.blueprint_id.clone()))
            .collect()
    }

    // TODO: move JSON serialization to builtins
    pub fn list_services(&self) -> Vec<JValue> {
        let services = self.services.read();
        let services = services
            .iter()
            .map(|(id, srv)| {
                json!({
                    "id": id,
                    "blueprint_id": srv.blueprint_id,
                    "owner_id": srv.owner_id.to_string(),
                    "aliases": srv.aliases
                })
            })
            .collect();

        services
    }

    // TODO: move JSON serialization to builtins
    pub fn get_service_mem_stats(&self, service_id: String) -> Result<Vec<JValue>, JError> {
        let services = self.services.read();
        let (service, _) = get_service(&services, &self.aliases.read(), service_id)
            .map_err(ServiceError::NoSuchService)?;

        let mut lock = service.service.lock();
        let stats = lock.module_memory_stats();
        let stats = stats
            .0
            .into_iter()
            .map(|stat| {
                json!({
                    "name": stat.name,
                    "memory_size_bytes": stat.memory_size,
                    "max_memory_size_bytes": stat.max_memory_size
                })
            })
            .collect();

        Ok(stats)
    }

    fn create_persisted_services(&self) {
        let services = load_persisted_services(&self.config.services_dir).into_iter();
        let services = services.filter_map(|r| match r {
            Ok(service) => service.into(),
            Err(err) => {
                log::warn!("Error loading one of persisted services: {:?}", err);
                None
            }
        });

        for s in services {
            let start = Instant::now();
            let result = self.create_service_inner(
                s.blueprint_id,
                s.owner_id,
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
            let mut aliases = self.aliases.write();
            for alias in s.aliases.into_iter() {
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

            log::info!(
                "Persisted service {} created in {}",
                s.service_id,
                pretty(start.elapsed())
            );
        }
    }

    fn create_service_inner(
        &self,
        blueprint_id: String,
        owner_id: PeerId,
        service_id: String,
        aliases: Vec<String>,
    ) -> Result<Option<Service>, ServiceError> {
        let creation_start_time = Instant::now();
        let mut service = create_app_service(
            self.config.clone(),
            &self.modules,
            blueprint_id.clone(),
            service_id.clone(),
            aliases.clone(),
            owner_id,
            self.metrics.as_ref(),
        )
        .inspect_err(|_| {
            if let Some(metrics) = self.metrics.as_ref() {
                metrics.observe_external(|external| {
                    external.creation_failure_count.inc();
                })
            }
        })?;
        let stats = service.module_memory_stats();
        let stats = ServiceMemoryStat::new(&stats);
        let service_type = ServiceType::Service(aliases.first().cloned());
        let service = Service {
            service: Mutex::new(service),
            blueprint_id,
            owner_id,
            aliases,
        };

        let replaced = self.services.write().insert(service_id.clone(), service);
        let creation_end_time = creation_start_time.elapsed().as_secs();
        if let Some(m) = self.metrics.as_ref() {
            m.observe_created(service_id, service_type, stats, creation_end_time as f64);
        }

        Ok(replaced)
    }

    fn create_vault(&self, particle_id: &str) -> Result<(), VaultError> {
        self.vault.create(particle_id)
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
    use libp2p_core::identity::Keypair;
    use libp2p_core::PeerId;
    use tempdir::TempDir;

    use config_utils::{modules_dir, to_peer_id};
    use fluence_libp2p::RandomPeerId;
    use particle_modules::{AddBlueprint, ModuleRepository};
    use server_config::ServicesConfig;
    use service_modules::load_module;
    use service_modules::{Dependency, Hash};

    use crate::persistence::load_persisted_services;
    use crate::{ParticleAppServices, ServiceError};

    fn create_pid() -> PeerId {
        let keypair = Keypair::generate_ed25519();

        PeerId::from(keypair.public())
    }

    fn create_pas(
        local_pid: PeerId,
        management_pid: PeerId,
        base_dir: PathBuf,
    ) -> ParticleAppServices {
        let startup_kp = Keypair::generate_ed25519();
        let vault_dir = base_dir.join("..").join("vault");
        let max_heap_size = server_config::default_module_max_heap_size();
        let config = ServicesConfig::new(
            local_pid,
            base_dir,
            vault_dir,
            HashMap::new(),
            management_pid,
            to_peer_id(&startup_kp),
            max_heap_size,
            None,
        )
        .unwrap();

        let repo = ModuleRepository::new(
            &config.modules_dir,
            &config.blueprint_dir,
            &config.particles_vault_dir,
            max_heap_size,
            None,
        );

        ParticleAppServices::new(config, repo, None)
    }

    fn call_add_alias_raw(
        as_manager: bool,
        alias: String,
        service_id: String,
    ) -> Result<(), ServiceError> {
        let base_dir = TempDir::new("test3").unwrap();
        let local_pid = create_pid();
        let management_pid = create_pid();
        let pas = create_pas(local_pid, management_pid, base_dir.into_path());

        let client_pid;
        if as_manager {
            client_pid = management_pid;
        } else {
            client_pid = create_pid();
        }

        pas.add_alias(alias, service_id, client_pid)
    }

    fn call_add_alias(alias: String, service_id: String) -> Result<(), ServiceError> {
        call_add_alias_raw(true, alias, service_id)
    }

    fn create_service(
        pas: &ParticleAppServices,
        module_name: String,
        module: &str,
    ) -> Result<String, String> {
        let dep = Dependency::Hash(Hash::from_hex(module).unwrap());
        let bp = pas
            .modules
            .add_blueprint(AddBlueprint::new(module_name, vec![dep]))
            .unwrap();

        pas.create_service(bp, RandomPeerId::random())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn test_add_alias_forbidden() {
        let resp = call_add_alias_raw(false, "1".to_string(), "2".to_string());
        assert!(resp.is_err());
        assert!(matches!(
            resp.err().unwrap(),
            ServiceError::Forbidden { .. }
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
        let local_pid = create_pid();
        let management_pid = create_pid();
        let base_dir = TempDir::new("test").unwrap();
        let pas = create_pas(local_pid, management_pid, base_dir.path().into());

        let module = load_module(
            "../crates/particle-node-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let module_name = "tetra".to_string();
        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: module_name.clone(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                mem_pages_count: None,
                max_heap_size: None,
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };
        let hash = pas
            .modules
            .add_module_base64(base64.encode(module), config)
            .unwrap();
        let service_id1 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id2 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id3 = create_service(&pas, module_name, &hash).unwrap();

        let inter1 = pas.get_interface(service_id1).unwrap();

        // delete module and check that interfaces will be returned anyway
        let dir = modules_dir(base_dir.path());
        let module_file = dir.join(format!("{hash}.wasm"));
        remove_file(module_file.clone()).unwrap();

        let inter2 = pas.get_interface(service_id2).unwrap();
        let inter3 = pas.get_interface(service_id3).unwrap();

        assert_eq!(module_file.exists(), false);
        assert_eq!(inter1, inter2);
        assert_eq!(inter3, inter2);
    }

    fn upload_tetra_service(pas: &ParticleAppServices, module_name: String) -> String {
        let module = load_module(
            "../crates/particle-node-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: module_name,
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                mem_pages_count: None,
                max_heap_size: None,
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
        let local_pid = create_pid();
        let management_pid = create_pid();
        let pas = create_pas(local_pid, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let hash = upload_tetra_service(&pas, module_name.clone());
        let service_id1 = create_service(&pas, module_name, &hash).unwrap();

        let alias = "alias";
        let result = pas.add_alias(alias.to_string(), service_id1.clone(), management_pid);
        // result of the add_alias call must be ok
        assert!(result.is_ok());

        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();
        // the service's alias list must contain the alias
        assert_eq!(service_1.aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
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
        let local_pid = create_pid();
        let management_pid = create_pid();
        let pas = create_pas(local_pid, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id2 = create_service(&pas, module_name, &hash).unwrap();

        let alias = "alias";
        // add an alias to a service
        let _ = pas.add_alias(alias.to_string(), service_id1.clone(), management_pid);
        // give the alias to another service
        let _ = pas.add_alias(alias.to_string(), service_id2.clone(), management_pid);

        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();
        let service_2 = services.get(&service_id2).unwrap();
        // the first service's alias list must not contain the alias
        assert_eq!(service_1.aliases, Vec::<String>::new());
        // the second service's alias list must contain the alias
        assert_eq!(service_2.aliases, vec![alias.to_string()]);

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
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
    fn test_persisted_service() {
        let base_dir = TempDir::new("test4").unwrap();
        let local_pid = create_pid();
        let management_pid = create_pid();
        let pas = create_pas(local_pid, management_pid, base_dir.into_path());

        let module_name = "tetra".to_string();
        let hash = upload_tetra_service(&pas, module_name.clone());

        let service_id1 = create_service(&pas, module_name, &hash).unwrap();
        let services = pas.services.read();
        let service_1 = services.get(&service_id1).unwrap();

        let persisted_services: Vec<_> = load_persisted_services(&pas.config.services_dir)
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();
        let persisted_service_1 = persisted_services.first().unwrap();
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
