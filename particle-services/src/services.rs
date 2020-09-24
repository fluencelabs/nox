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
use std::collections::HashMap;
use std::sync::Arc;

type XAppService = Arc<Mutex<AppService>>;

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

    pub fn create_service(
        &self,
    ) -> impl Fn(String, String) -> Result<String, ServiceError> + Send + 'static {
        let services = self.services.clone();
        let config = self.config.clone();
        move |blueprint_id, user_id| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let vm = create_vm(config.clone(), blueprint_id, &service_id, user_id.into());
            match vm {
                Ok(vm) => {
                    let vm = Arc::new(Mutex::new(vm));
                    services.write().insert(service_id.clone(), vm);
                    Ok(service_id)
                }
                Err(e) => Err(e),
            }
        }
    }

    pub fn call_service(
        &self,
    ) -> impl Fn(String, String, String, serde_json::Value) -> Result<Vec<IValue>, ServiceError>
           + Send
           + 'static {
        let services = self.services.clone();
        move |service_id, module, fname, args| {
            if let Some(vm) = services.read().get(&service_id) {
                let result = vm.lock().call(module, fname, args, <_>::default());
                result.map_err(ServiceError::Engine)
            } else {
                Err(ServiceError::NoSuchInstance(service_id))
            }
        }
    }
}
