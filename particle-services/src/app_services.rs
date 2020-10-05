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
use crate::error::ServiceError::MissingBlueprintId;
use crate::vm::create_vm;

use fluence_app_service::{AppService, IValue};
use host_closure::Closure;

use parking_lot::{Mutex, RwLock};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

type VM = Arc<Mutex<AppService>>;
type Services = Arc<RwLock<HashMap<String, VM>>>;

pub struct ParticleAppServices {
    config: ServicesConfig,
    services: Services,
}

impl ParticleAppServices {
    pub fn new(config: ServicesConfig) -> Self {
        Self {
            config,
            services: <_>::default(),
        }
    }

    pub fn create_service(&self) -> Closure {
        let services = self.services.clone();
        let config = self.config.clone();

        Arc::new(move |args| {
            let args = args.args;
            let service_id = uuid::Uuid::new_v4().to_string();
            let make_vm = || {
                let blueprint_id = args
                    .get("blueprint_id")
                    .and_then(|v| v.as_str())
                    .ok_or(MissingBlueprintId)?
                    .to_string();

                let user_id = args
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string());
                create_vm(config.clone(), blueprint_id, &service_id, user_id)
            };

            match make_vm() {
                Ok(vm) => {
                    let vm = Arc::new(Mutex::new(vm));
                    services.write().insert(service_id.clone(), vm);
                    ivalue_utils::ok(json!(service_id))
                }
                Err(err) => ivalue_utils::error(json!(err.to_string())),
            }
        })
    }

    pub fn call_service(&self) -> Closure {
        let services = self.services.clone();

        Arc::new(move |args| {
            let call = || -> Result<Vec<IValue>, ServiceError> {
                let services = services.read();
                let vm = services
                    .get(&args.service_id)
                    .ok_or(ServiceError::NoSuchInstance(args.service_id))?;
                let result = vm
                    .lock()
                    .call("facade".to_string(), args.fname, args.args, <_>::default())
                    .map_err(ServiceError::Engine)?;

                Ok(result)
            };

            match call() {
                // AppService always returns a single element
                Ok(result) => {
                    let result = result.into_iter().next().expect("must be defined");
                    ivalue_utils::ivalue_ok(result)
                }
                Err(err) => ivalue_utils::error(json!(err.to_string())),
            }
        })
    }
}
