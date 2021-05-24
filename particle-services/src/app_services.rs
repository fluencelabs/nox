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

use fluence_app_service::{AppService, CallParameters, ServiceInterface};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Value as JValue};

use host_closure::{closure, closure_params, closure_params_opt, Args, Closure, ParticleClosure};
use particle_modules::ModuleRepository;
use server_config::ServicesConfig;

use crate::app_service::create_app_service;
use crate::error::ServiceError;
use crate::error::ServiceError::{AliasAsServiceId, Forbidden, NoSuchAlias};
use crate::persistence::{load_persisted_services, persist_service, PersistedService};

type Services = Arc<RwLock<HashMap<String, Service>>>;
type Aliases = Arc<RwLock<HashMap<String, String>>>;

pub struct Service {
    pub service: Mutex<AppService>,
    pub blueprint_id: String,
    pub owner_id: String,
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

#[derive(Serialize)]
pub struct VmDescriptor<'a> {
    interface: ServiceInterface,
    blueprint_id: &'a str,
    service_id: &'a str,
    owner_id: &'a str,
}

pub struct ParticleAppServices {
    config: ServicesConfig,
    services: Services,
    modules: ModuleRepository,
    aliases: Aliases,
    management_peer_id: String,
}

pub fn get_service<'l>(
    services: &'l HashMap<String, Service>,
    aliases: &HashMap<String, String>,
    id_or_alias: String,
) -> Result<(&'l Service, String), ServiceError> {
    services
        .get(&id_or_alias)
        .map(|s| (s, id_or_alias.clone()))
        .or_else(|| {
            aliases
                .get(&id_or_alias)
                .and_then(|id| (services.get(id)).map(|s| (s, id.clone())))
        })
        .ok_or_else(|| ServiceError::NoSuchService(id_or_alias.clone()))
}

impl ParticleAppServices {
    pub fn new(config: ServicesConfig, modules: ModuleRepository) -> Self {
        let management_peer_id = config.management_peer_id.to_base58();
        let this = Self {
            config,
            services: <_>::default(),
            modules,
            aliases: <_>::default(),
            management_peer_id,
        };

        this.create_persisted_services();

        this
    }

    pub fn create_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let config = self.config.clone();
        let modules = self.modules.clone();

        closure_params(move |particle, args| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let blueprint_id: String =
                Args::next("blueprint_id", &mut args.function_args.into_iter())?;

            let service = create_app_service(
                config.clone(),
                &modules,
                blueprint_id.clone(),
                service_id.clone(),
                vec![],
                particle.init_user_id.clone(),
            )?;
            let service = Service {
                service: Mutex::new(service),
                blueprint_id,
                owner_id: particle.init_user_id,
                aliases: vec![],
            };

            services.write().insert(service_id.clone(), service);

            Ok(json!(service_id))
        })
    }

    pub fn remove_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let aliases = self.aliases.clone();

        closure_params_opt(move |particle_params, args| {
            let mut args = args.function_args.into_iter();
            let service_id_or_alias: String = Args::next("service_id_or_alias", &mut args)?;
            let services_read = services.read();
            let (service, service_id) =
                get_service(&services_read, &aliases.read(), service_id_or_alias)?;
            if service.owner_id != particle_params.init_user_id {
                Err(ServiceError::Forbidden {
                    user: particle_params.init_user_id,
                    function: "remove_service",
                    reason: "only creator can remove service",
                })?;
            }

            services.write().remove(&service_id);

            Ok(None)
        })
    }

    pub fn call_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let aliases = self.aliases.clone();
        let host_id = self.config.local_peer_id.to_string();

        closure_params(move |particle_params, args| {
            let result: eyre::Result<_> = try {
                let services = services.read();
                let aliases = aliases.read();

                let (service, id) = get_service(&services, &aliases, args.service_id)?;

                let params = CallParameters {
                    host_id: host_id.clone(),
                    init_peer_id: particle_params.init_user_id,
                    particle_id: particle_params.particle_id,
                    tetraplets: args.tetraplets,
                    service_id: id,
                    service_creator_peer_id: service.owner_id.clone(),
                };

                let mut service = service.lock();
                service
                    .call(
                        args.function_name,
                        JValue::Array(args.function_args),
                        params,
                    )
                    .map_err(ServiceError::Engine)?
            };

            result.map_err(|err| {
                log::warn!("call_service error: {:?}", err);
                json!(format!("{:?}", err)
                    // TODO: send patch to eyre so it can be done through their API
                    // Remove backtrace from the response
                    .split("Stack backtrace:")
                    .next()
                    .unwrap_or_default())
            })
        })
    }

    pub fn add_alias(&self) -> ParticleClosure {
        let services = self.services.clone();
        let aliases = self.aliases.clone();
        let config = self.config.clone();
        let management_peer_id = self.management_peer_id.clone();

        closure_params_opt(move |particle, args| {
            if particle.init_user_id != management_peer_id {
                return Err(Forbidden {
                    user: particle.init_user_id,
                    function: "add_alias",
                    reason: "only management peer id can add aliases",
                }
                .into());
            };

            let mut args = args.function_args.into_iter();
            let alias: String = Args::next("alias", &mut args)?;
            let service_id: String = Args::next("service_id", &mut args)?;

            // if a client trying to add an alias that equals some created service id
            // return an error
            if services.read().get(&alias).is_some() {
                return Err(AliasAsServiceId(alias).into());
            }

            let mut services = services.write();

            let service = services
                .get_mut(&service_id)
                .ok_or_else(|| ServiceError::NoSuchService(service_id.clone()))?;
            service.add_alias(alias.clone());
            let persisted_new = PersistedService::from_service(service_id.clone(), service);

            let old_id = {
                let lock = aliases.read();
                lock.get(&alias).cloned()
            };

            let old = old_id.and_then(|s_id| services.get_mut(&s_id));
            let old = old.map(|old| {
                old.remove_alias(&alias);
                PersistedService::from_service(service_id.clone(), old)
            });

            drop(services);
            if let Some(old) = old {
                persist_service(&config.services_dir, old)?;
            }
            persist_service(&config.services_dir, persisted_new)?;

            aliases.write().insert(alias, service_id.clone());
            Ok(None)
        })
    }

    pub fn resolve_alias(&self) -> Closure {
        let aliases = self.aliases.clone();

        closure(move |mut args| {
            let alias: String = Args::next("alias", &mut args)?;
            let aliases = aliases.read();
            let service_id = aliases.get(&alias);

            service_id
                .ok_or(NoSuchAlias(alias))
                .map(|value| json!(value))
                .map_err(|err| {
                    log::warn!("call_service error: {:?}", err);
                    json!(format!("{:?}", err)
                        // TODO: send patch to eyre so it can be done through their API
                        // Remove backtrace from the response
                        .split("Stack backtrace:")
                        .next()
                        .unwrap_or_default())
                })
        })
    }

    pub fn get_interface(&self) -> Closure {
        let services = self.services.clone();
        let aliases = self.aliases.clone();
        let modules = self.modules.clone();

        closure(move |mut args| {
            let services = services.read();
            let service_id: String = Args::next("service_id", &mut args)?;
            let (service, _) = get_service(&services, &aliases.read(), service_id)?;

            Ok(modules.get_facade_interface(&service.blueprint_id)?)
        })
    }

    pub fn list_services(&self) -> Closure {
        let services = self.services.clone();

        closure(move |_| {
            let services = services.read();
            let services = services
                .iter()
                .map(|(id, srv)| {
                    json!({
                        "id": id,
                        "blueprint_id": srv.blueprint_id,
                        "owner_id": srv.owner_id,
                        "aliases": srv.aliases
                    })
                })
                .collect();

            Ok(services)
        })
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
                aliases: s.aliases,
            };
            let replaced = self.services.write().insert(s.service_id.clone(), service);

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );

            log::info!("Persisted service {} created", s.service_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use libp2p_core::identity::Keypair;
    use libp2p_core::PeerId;
    use serde_json::Value as JValue;
    use tempdir::TempDir;

    use crate::ParticleAppServices;
    use config_utils::modules_dir;
    use fluence_app_service::{TomlFaaSModuleConfig, TomlFaaSNamedModuleConfig};
    use host_closure::ParticleParameters;
    use particle_modules::{Dependency, Hash, ModuleRepository};
    use server_config::ServicesConfig;
    use std::fs::remove_file;
    use std::path::PathBuf;
    use test_utils::{
        add_bp, add_module, create_args, load_module, response_to_return, string_result, RetStruct,
    };

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
        let config =
            ServicesConfig::new(local_pid, base_dir, HashMap::new(), management_pid).unwrap();

        let repo = ModuleRepository::new(&config.modules_dir, &config.blueprint_dir);

        ParticleAppServices::new(config, repo)
    }

    fn params(pid: PeerId) -> ParticleParameters {
        ParticleParameters {
            init_user_id: pid.to_base58(),
            particle_id: "".to_string(),
        }
    }

    fn call_add_alias_raw(as_manager: bool, args: Vec<JValue>) -> RetStruct {
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

        let params = params(client_pid);
        let args = create_args(args);

        let resp = pas.add_alias()(params, args);
        response_to_return(resp.unwrap())
    }

    fn call_add_alias(args: Vec<JValue>) -> RetStruct {
        call_add_alias_raw(true, args)
    }

    fn create_service(
        pas: &ParticleAppServices,
        module_name: String,
        module: &str,
    ) -> Result<String, String> {
        let dep = Dependency::Hash(Hash::from_hex(module).unwrap());
        let bp = add_bp(&pas.modules, module_name, vec![dep]).unwrap();

        let args = create_args(vec![JValue::String(bp)]);

        let particle: ParticleParameters = ParticleParameters {
            init_user_id: "".to_string(),
            particle_id: "".to_string(),
        };

        let resp = pas.create_service()(particle, args).unwrap();
        string_result(response_to_return(resp))
    }

    fn get_interface(pas: &ParticleAppServices, id: String) -> RetStruct {
        let args = create_args(vec![JValue::String(id)]);
        let ret = pas.get_interface()(args).unwrap();
        response_to_return(ret)
    }

    #[test]
    fn test_add_alias_forbidden() {
        let resp = call_add_alias_raw(
            false,
            vec![
                JValue::String("1".to_string()),
                JValue::String("2".to_string()),
            ],
        );
        assert_eq!(resp.ret_code, 1);
        assert_eq!(true, resp.error.contains("Forbidden"))
    }

    #[test]
    fn test_add_alias_no_service() {
        let resp = call_add_alias(vec![
            JValue::String("1".to_string()),
            JValue::String("2".to_string()),
        ]);
        assert_eq!(resp.ret_code, 1);
        assert!(
            resp.error.contains("Service with id") && resp.error.contains("not found"),
            "Closure should not found a service to add alias `{}`",
            resp.error
        );
    }

    #[test]
    fn test_get_interface_cache() {
        let local_pid = create_pid();
        let management_pid = create_pid();
        let base_dir = TempDir::new("test").unwrap();
        let pas = create_pas(local_pid, management_pid, base_dir.path().into());

        let module = load_module("../particle-node/tests/tetraplets/artifacts", "tetraplets");

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
        let hash = add_module(&pas.modules, base64::encode(module), config).unwrap();
        let service_id1 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id2 = create_service(&pas, module_name.clone(), &hash).unwrap();
        let service_id3 = create_service(&pas, module_name.clone(), &hash).unwrap();

        let inter1 = get_interface(&pas, service_id1);

        // delete module and check that interfaces will be returned anyway
        let dir = modules_dir(base_dir.path().into());
        let module_file = dir.join(format!("{}.wasm", hash));
        remove_file(module_file.clone()).unwrap();

        let inter2 = get_interface(&pas, service_id2);
        let inter3 = get_interface(&pas, service_id3);

        assert_eq!(module_file.exists(), false);
        assert_eq!(inter1.result, inter2.result);
        assert_eq!(inter3.result, inter2.result);
    }

    // TODO: add more tests
    //       - add alias success & fail with service collision & test on rewriting alias
    //       - create_service success & fail
    //       - get_modules success & fail
    //       - get_interface
    //       - list_services
    //       - test on service persisting
}
