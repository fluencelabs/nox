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
use std::{collections::HashMap, sync::Arc};

use derivative::Derivative;
use fluence_app_service::{AppService, CallParameters, ServiceInterface};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Value as JValue};

use fluence_libp2p::PeerId;
use particle_args::Args;
use particle_execution::{FunctionOutcome, ParticleParams, ParticleVault, VaultError};
use particle_modules::ModuleRepository;
use server_config::ServicesConfig;

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
    pub service: Mutex<AppService>,
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
    pub fn new(config: ServicesConfig, modules: ModuleRepository) -> Self {
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

        let service = create_app_service(
            self.config.clone(),
            &self.modules,
            blueprint_id.clone(),
            service_id.clone(),
            vec![],
            init_peer_id.clone(),
        )?;
        let service = Service {
            service: Mutex::new(service),
            blueprint_id,
            owner_id: init_peer_id,
            aliases: vec![],
        };

        self.services.write().insert(service_id.clone(), service);

        Ok(service_id)
    }

    pub fn remove_service(
        &self,
        service_id_or_alias: String,
        init_peer_id: PeerId,
    ) -> Result<(), ServiceError> {
        let service_id = {
            let services_read = self.services.read();
            let (service, service_id) =
                get_service(&services_read, &self.aliases.read(), service_id_or_alias)
                    .map_err(|id| ServiceError::NoSuchService(id))?;

            if service.owner_id != init_peer_id && self.builtins_management_peer_id != init_peer_id
            {
                return Err(ServiceError::Forbidden {
                    user: init_peer_id,
                    function: "remove_service",
                    reason: "only creator can remove service",
                });
            }

            service_id
        };

        remove_persisted_service(&self.config.services_dir, service_id.clone()).unwrap();
        let service = self.services.write().remove(&service_id).unwrap();
        let mut aliases = self.aliases.write();
        for alias in service.aliases.iter() {
            aliases.remove(alias);
        }

        Ok(())
    }

    pub fn call_service(
        &self,
        mut function_args: Args,
        particle: ParticleParams,
    ) -> FunctionOutcome {
        let services = self.services.read();
        let aliases = self.aliases.read();
        let host_id = self.config.local_peer_id.to_string();

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

        // TODO: move particle vault creation to aquamarine::particle_functions
        self.create_vault(&particle.id)?;

        let params = CallParameters {
            host_id,
            particle_id: particle.id,
            init_peer_id: particle.init_peer_id.to_string(),
            tetraplets: function_args.tetraplets,
            service_id,
            service_creator_peer_id: service.owner_id.to_string(),
        };
        let function_name = function_args.function_name;

        let mut service = service.lock();
        let result = service
            .call(
                function_name,
                JValue::Array(function_args.function_args),
                params,
            )
            .map_err(ServiceError::Engine)?;

        FunctionOutcome::Ok(result)
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

        let old_id = {
            let lock = self.aliases.read();
            lock.get(&alias).cloned()
        };

        let old = old_id.and_then(|s_id| services.get_mut(&s_id));
        let old = old.map(|old| {
            old.remove_alias(&alias);
            PersistedService::from_service(service_id.clone(), old)
        });

        drop(services);
        if let Some(old) = old {
            persist_service(&self.config.services_dir, old)?;
        }
        persist_service(&self.config.services_dir, persisted_new)?;

        self.aliases.write().insert(alias, service_id.clone());

        Ok(())
    }

    pub fn resolve_alias(&self, alias: String) -> Result<String, ServiceError> {
        let aliases = self.aliases.read();
        let service_id = aliases.get(&alias);

        service_id.cloned().ok_or(NoSuchAlias(alias))
    }

    pub fn get_interface(&self, service_id: String) -> Result<JValue, ServiceError> {
        let services = self.services.read();
        let (service, _) = get_service(&services, &self.aliases.read(), service_id)
            .map_err(|id| ServiceError::NoSuchService(id))?;

        Ok(self.modules.get_facade_interface(&service.blueprint_id)?)
    }

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
            let service = create_app_service(
                self.config.clone(),
                &self.modules,
                s.blueprint_id.clone(),
                s.service_id.clone(),
                s.aliases.clone(),
                s.owner_id.clone(),
            );
            let service = match service {
                Ok(service) => service,
                Err(err) => {
                    #[rustfmt::skip]
                    log::warn!("Error creating service for persisted service {}: {:#?}", s.service_id, err);
                    continue;
                }
            };

            let service = Service {
                service: Mutex::new(service),
                blueprint_id: s.blueprint_id,
                owner_id: s.owner_id,
                aliases: s.aliases.clone(),
            };

            let replaced = self.services.write().insert(s.service_id.clone(), service);
            let mut aliases = self.aliases.write();
            for alias in s.aliases.into_iter() {
                aliases.insert(alias, s.service_id.clone());
            }

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );

            log::info!("Persisted service {} created", s.service_id);
        }
    }

    fn create_vault(&self, particle_id: &str) -> Result<(), VaultError> {
        self.vault.create(particle_id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::remove_file;
    use std::path::PathBuf;

    use fluence_app_service::{TomlFaaSModuleConfig, TomlFaaSNamedModuleConfig};
    use libp2p_core::identity::Keypair;
    use libp2p_core::PeerId;
    use tempdir::TempDir;

    use config_utils::{modules_dir, to_peer_id};
    use fluence_libp2p::RandomPeerId;
    use particle_modules::{AddBlueprint, ModuleRepository};
    use server_config::ServicesConfig;
    use service_modules::load_module;
    use service_modules::{Dependency, Hash};

    use crate::{ParticleAppServices, ServiceError};

    fn create_pid() -> PeerId {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        peer_id
    }

    fn create_pas(
        local_pid: PeerId,
        management_pid: PeerId,
        base_dir: PathBuf,
    ) -> ParticleAppServices {
        let startup_kp = Keypair::generate_ed25519();
        let vault_dir = base_dir.join("..").join("vault");
        let config = ServicesConfig::new(
            local_pid,
            base_dir.clone(),
            vault_dir.clone(),
            HashMap::new(),
            management_pid,
            to_peer_id(&startup_kp),
        )
        .unwrap();

        let repo = ModuleRepository::new(
            &config.modules_dir,
            &config.blueprint_dir,
            &config.particles_vault_dir,
        );

        ParticleAppServices::new(config, repo)
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
            client_pid = management_pid.clone();
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

        let module = load_module("../particle-node/tests/tetraplets/artifacts", "tetraplets")
            .expect("load module");

        let module_name = "tetra".to_string();
        let config: TomlFaaSNamedModuleConfig = TomlFaaSNamedModuleConfig {
            name: module_name.clone(),
            file_name: None,
            config: TomlFaaSModuleConfig {
                mem_pages_count: None,
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };
        let hash = pas
            .modules
            .add_module_base64(base64::encode(module), config)
            .unwrap();
        let service_id1 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id2 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id3 = create_service(&pas, module_name.clone(), &hash).unwrap();

        let inter1 = pas.get_interface(service_id1).unwrap();

        // delete module and check that interfaces will be returned anyway
        let dir = modules_dir(base_dir.path().into());
        let module_file = dir.join(format!("{}.wasm", hash));
        remove_file(module_file.clone()).unwrap();

        let inter2 = pas.get_interface(service_id2).unwrap();
        let inter3 = pas.get_interface(service_id3).unwrap();

        assert_eq!(module_file.exists(), false);
        assert_eq!(inter1, inter2);
        assert_eq!(inter3, inter2);
    }

    // TODO: add more tests
    //       - add alias success & fail with service collision & test on rewriting alias
    //       - create_service success & fail
    //       - get_modules success & fail
    //       - get_interface
    //       - list_services
    //       - test on service persisting
}
