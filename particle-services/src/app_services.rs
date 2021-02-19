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

use crate::error::ServiceError;
use crate::persistence::load_persisted_services;
use crate::app_service::create_app_service;

use fluence_app_service::{AppService, CallParameters, ServiceInterface};
use host_closure::{closure, closure_args, closure_params, Args, Closure, ParticleClosure};
use server_config::ServicesConfig;

use parking_lot::{Mutex, RwLock};
use particle_modules::ModuleRepository;
use serde::Serialize;
use serde_json::{json, Value as JValue};
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};

type Services = Arc<RwLock<HashMap<String, Service>>>;
type Aliases = Arc<RwLock<HashMap<String, String>>>;

pub struct Service {
    service: Mutex<AppService>,
    blueprint_id: String,
    owner_id: String,
    aliases: Vec<String>
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
    #[serde(skip_serializing_if = "Option::is_none")]
    service_id: Option<&'a str>,
    owner_id: &'a str,
}

pub struct ParticleAppServices {
    config: ServicesConfig,
    services: Services,
    modules: ModuleRepository,
    aliases: Aliases
}

impl ParticleAppServices {
    pub fn new(config: ServicesConfig, modules: ModuleRepository) -> Self {
        let this = Self {
            config,
            services: <_>::default(),
            modules,
            aliases: <_>::default(),
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
                aliases: vec![]
            };

            services.write().insert(service_id.clone(), service);

            Ok(json!(service_id))
        })
    }

    pub fn call_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let host_id = self.config.local_peer_id.to_string();

        closure_params(move |particle_params, args| {
            let services = services.read();
            let service = services
                .get(&args.service_id)
                .ok_or_else(|| ServiceError::NoSuchInstance(args.service_id.clone()))?;

            let params = CallParameters {
                host_id: host_id.clone(),
                init_peer_id: particle_params.init_user_id,
                particle_id: particle_params.particle_id,
                tetraplets: args.tetraplets,
                service_id: args.service_id,
                service_creator_peer_id: service.owner_id.clone(),
            };

            let result = service
                .lock()
                .call(
                    args.function_name,
                    JValue::Array(args.function_args),
                    params,
                )
                .map_err(ServiceError::Engine)?;

            Ok(result)
        })
    }

    // add_alias(alias, service_id):
    //   let mut current_alias = self.aliases.remove(alias);
    //   current_alias.remove_alias(alias); // self.aliases.retain(|a| a != alias)
    //   let service = self.services.read().get(&service_id)...
    //   let mut service = service.clone();
    //   persist_service(dir, current_alias.id, current_alias.blueprint_id, current_alias.owner_id, current_alias.aliases)
    //   persist_service(dir, service_id, service.blueprint_id, service.owner_id, service.aliases +: alias)
    //   service.add_alias(alias)
    //   self.aliases.insert(alias, service)

    pub fn add_alias(&self) -> ParticleClosure {
        let services = self.services.clone();
        let aliases = self.aliases.clone();
        let host_id = self.config.local_peer_id.to_string();
        println!("{:?}", host_id);

        closure_params(move |particle, args| {
            let mut args = args.function_args.into_iter();
            let aliases = aliases.read();

            let alias: String =
                Args::next("alias", &mut args)?;

            let service_id: String =
                Args::next("service_id", &mut args)?;

            let old_service_id = aliases.get(&alias);

            println!("{:?}", old_service_id);


            // services.write().insert(service_id.clone(), service);

            Ok(json!(service_id))
        })
    }

    pub fn get_interface(&self) -> Closure {
        let services = self.services.clone();

        closure_args(move |args| {
            let services = services.read();
            let service = services
                .get(&args.service_id)
                .ok_or(ServiceError::NoSuchInstance(args.service_id))?;

            Ok(get_service_interface(service, None)?)
        })
    }

    pub fn get_active_interfaces(&self) -> Closure {
        let services = self.services.clone();

        closure(move |_| {
            let services = services.read();
            let interfaces = services
                .iter()
                .map(|(id, service)| match get_service_interface(service, id.as_str().into()) {
                    Ok(iface) => iface,
                    Err(err) => json!({ "service_id": id, "error": JValue::from(err)}),
                })
                .collect();

            Ok(interfaces)
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
                aliases: s.aliases
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

fn get_service_interface(service: &Service, service_id: Option<&str>) -> Result<JValue, ServiceError> {
    let lock = service.lock();
    let interface = lock.get_interface();

    let descriptor = VmDescriptor {
        interface,
        blueprint_id: &service.blueprint_id,
        service_id,
        owner_id: &service.owner_id,
    };
    let descriptor =
        serde_json::to_value(descriptor).map_err(ServiceError::CorruptedFaaSInterface)?;

    Ok(descriptor)
}
