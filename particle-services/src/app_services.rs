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

use crate::config::ServicesConfig;
use crate::error::ServiceError;
use crate::persistence::load_persisted_services;
use crate::vm::create_vm;

use fluence_app_service::{AppService, CallParameters, ServiceInterface};
use host_closure::{closure, closure_args, closure_params, Args, Closure, ParticleClosure};

use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::{json, Value as JValue};
use std::ops::Deref;
use std::{collections::HashMap, sync::Arc};

type Services = Arc<RwLock<HashMap<String, Service>>>;

pub struct Service {
    vm: Arc<Mutex<AppService>>,
    blueprint_id: String,
    owner_id: String,
}

impl Deref for Service {
    type Target = Arc<Mutex<AppService>>;

    fn deref(&self) -> &Self::Target {
        &self.vm
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
}

impl ParticleAppServices {
    pub fn new(config: ServicesConfig) -> Self {
        let this = Self {
            config,
            services: <_>::default(),
        };

        this.create_persisted_services();

        this
    }

    pub fn create_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let config = self.config.clone();

        closure_params(move |particle, args| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let blueprint_id: String =
                Args::next("blueprint_id", &mut args.function_args.into_iter())?;

            let vm = create_vm(
                config.clone(),
                blueprint_id.clone(),
                service_id.clone(),
                particle.init_user_id.clone(),
            )?;
            let vm = Arc::new(Mutex::new(vm));
            let vm = Service {
                vm,
                blueprint_id,
                owner_id: particle.init_user_id,
            };

            services.write().insert(service_id.clone(), vm);

            Ok(json!(service_id))
        })
    }

    pub fn call_service(&self) -> ParticleClosure {
        let services = self.services.clone();
        let host_id = self.config.current_peer_id.clone();

        Arc::new(move |particle_params, args| {
            let call = || -> Result<JValue, ServiceError> {
                let services = services.read();
                let vm = services
                    .get(&args.service_id)
                    .ok_or_else(|| ServiceError::NoSuchInstance(args.service_id.clone()))?;

                let params = CallParameters {
                    host_id: host_id.clone(),
                    init_peer_id: particle_params.init_user_id,
                    particle_id: particle_params.particle_id,
                    tetraplets: args.tetraplets,
                    service_id: args.service_id,
                    service_creator_peer_id: vm.owner_id.clone(),
                };

                let result = vm
                    .lock()
                    .call(
                        args.function_name,
                        JValue::Array(args.function_args),
                        params,
                    )
                    .map_err(ServiceError::Engine)?;

                Ok(result)
            };

            match call() {
                Ok(result) => ivalue_utils::ok(result),
                Err(err) => {
                    log::warn!("call_service error: {}", err);
                    ivalue_utils::error(json!(err.to_string()))
                }
            }
        })
    }

    pub fn get_interface(&self) -> Closure {
        let services = self.services.clone();

        closure_args(move |args| {
            let services = services.read();
            let vm = services
                .get(&args.service_id)
                .ok_or(ServiceError::NoSuchInstance(args.service_id))?;

            Ok(get_vm_interface(vm, None)?)
        })
    }

    pub fn get_active_interfaces(&self) -> Closure {
        let services = self.services.clone();

        closure(move |_| {
            let services = services.read();
            let interfaces = services
                .iter()
                .map(|(id, vm)| match get_vm_interface(vm, id.as_str().into()) {
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
            let owner_id = s.owner_id;
            let service_id = s.service_id.clone();
            let blueprint_id = s.blueprint_id.clone();
            let config = self.config.clone();
            let vm = match create_vm(config, blueprint_id, service_id, owner_id.clone()) {
                Ok(vm) => vm,
                Err(err) => {
                    #[rustfmt::skip]
                    log::warn!("Error creating vm for persisted service {}: {:#?}", s.service_id, err);
                    continue;
                }
            };

            let vm = Service {
                vm: Arc::new(Mutex::new(vm)),
                blueprint_id: s.blueprint_id,
                owner_id,
            };
            let replaced = self.services.write().insert(s.service_id.clone(), vm);

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );

            log::info!("Persisted service {} created", s.service_id);
        }
    }
}

fn get_vm_interface(vm: &Service, service_id: Option<&str>) -> Result<JValue, ServiceError> {
    let lock = vm.lock();
    let interface = lock.get_interface();

    let descriptor = VmDescriptor {
        interface,
        blueprint_id: &vm.blueprint_id,
        service_id,
        owner_id: &vm.owner_id,
    };
    let descriptor =
        serde_json::to_value(descriptor).map_err(ServiceError::CorruptedFaaSInterface)?;

    Ok(descriptor)
}
