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

use fluence_app_service::{AppService, IValue};

use crate::vm::create_vm;
use parking_lot::{Mutex, RwLock};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

type XAppService = Arc<Mutex<AppService>>;
type Callback =
    Box<dyn Fn(String, String, serde_json::Value) -> serde_json::Value + Send + 'static>;

pub struct ParticleServices {
    config: ServicesConfig,
    services: Arc<RwLock<HashMap<String, XAppService>>>,
}

impl ParticleServices {
    pub fn new(config: ServicesConfig) -> Self {
        Self {
            config,
            services: <_>::default(),
        }
    }

    pub fn create_service(&self) -> Callback {
        let services = self.services.clone();
        let config = self.config.clone();
        Box::new(move |_, _, args| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let make_vm = || {
                let blueprint_id = args
                    .get("blueprint_id")
                    .and_then(|v| v.as_str())
                    .ok_or(ServiceError::MissingBlueprintId)?
                    .to_string();
                let user_id = args
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                create_vm(config.clone(), blueprint_id, &service_id, user_id)
            };
            match make_vm() {
                Ok(vm) => {
                    let vm = Arc::new(Mutex::new(vm));
                    services.write().insert(service_id.clone(), vm);
                    json!({ "service_id": service_id })
                }
                Err(err) => json!({ "error": err.to_string() }),
            }
        })
    }

    pub fn call_service(&self) -> Callback {
        let services = self.services.clone();
        Box::new(move |service_id, fname, args| {
            let result = if let Some(vm) = services.read().get(&service_id) {
                let result = vm
                    .lock()
                    .call("facade".to_string(), fname, args, <_>::default());
                result.map_err(ServiceError::Engine)
            } else {
                Err(ServiceError::NoSuchInstance(service_id))
            };

            match result {
                Ok(result) => json!({ "result": unimplemented!("{:?}", result) }),
                Err(err) => json!({"error": err.to_string() }),
            }
        })
    }
}
