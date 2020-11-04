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

use fluence_app_service::AppService;
use host_closure::{closure, Args, Closure};
use json_utils::err_as_value;

use parking_lot::{Mutex, RwLock};
use serde_json::{json, Value as JValue};
use std::{collections::HashMap, sync::Arc};

type VM = Arc<Mutex<AppService>>;
type Services = Arc<RwLock<HashMap<String, VM>>>;

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

    pub fn create_service(&self) -> Closure {
        let services = self.services.clone();
        let config = self.config.clone();

        closure(move |mut args| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let mut make_vm = || {
                let blueprint_id: String = Args::next("blueprint_id", &mut args)?;
                let user_id = Args::maybe_next("user_id", &mut args)?;

                create_vm(config.clone(), blueprint_id, service_id.clone(), user_id)
            };

            make_vm().map_err(err_as_value).map(|vm| {
                let vm = Arc::new(Mutex::new(vm));
                services.write().insert(service_id.clone(), vm);
                json!(service_id)
            })
        })
    }

    pub fn call_service(&self) -> Closure {
        let services = self.services.clone();

        Arc::new(move |args| {
            let call = || -> Result<JValue, ServiceError> {
                let services = services.read();
                let vm = services
                    .get(&args.service_id)
                    .ok_or(ServiceError::NoSuchInstance(args.service_id))?;
                let result = vm
                    .lock()
                    .call(args.fname, JValue::Array(args.args), <_>::default())
                    .map_err(ServiceError::Engine)?;

                Ok(result)
            };

            match call() {
                Ok(result) => ivalue_utils::ok(result),
                Err(err) => ivalue_utils::error(json!(err.to_string())),
            }
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
            let owner_id = s.owner_id.map(|s| s.to_string());
            let service_id = s.service_id.clone();
            let config = self.config.clone();
            let vm = match create_vm(config, s.blueprint_id, service_id, owner_id) {
                Ok(vm) => vm,
                Err(err) => {
                    #[rustfmt::skip]
                    log::warn!("Error creating vm for persisted service {}: {:#?}", s.service_id, err);
                    continue;
                }
            };

            let vm = Arc::new(Mutex::new(vm));
            let replaced = self.services.write().insert(s.service_id.clone(), vm);

            debug_assert!(
                replaced.is_none(),
                "shouldn't replace any existing services"
            );

            log::info!("Persisted service {} created", s.service_id);
        }
    }
}
