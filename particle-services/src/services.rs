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

use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

type VM = Arc<Mutex<AppService>>;
type Callback = Box<dyn (Fn(Vec<IValue>) -> Option<IValue>) + Send + Sync + 'static>;
type Fabric = Arc<dyn Fn() -> Callback + Send + Sync + 'static>;
type Services = Arc<RwLock<HashMap<String, VM>>>;

pub struct ParticleServices {
    config: ServicesConfig,
    services: Services,
}

impl ParticleServices {
    pub fn new(config: ServicesConfig) -> Self {
        Self {
            config,
            services: <_>::default(),
        }
    }

    #[allow(dead_code)]
    fn spawn(&self) {
        use futures::executor::ThreadPool;
        use futures::task::SpawnExt;
        let executor = ThreadPool::new().unwrap();

        let fabric = self.fabric().clone();
        executor
            .spawn(async move {
                let cb = fabric();
                cb(<_>::default());
            })
            .expect("spawn");
    }

    pub fn fabric(&self) -> Fabric {
        let services = self.services.clone();
        let config = self.config.clone();

        Arc::new(move || {
            let create_service = Self::create_service(services.clone(), config.clone());
            let call_service = Self::call_service(services.clone());
            Box::new(move |args| {
                let mut args = args.into_iter().peekable();
                let name = args.peek().and_then(Self::into_str);
                // route
                match name {
                    Some("create") => create_service(args.collect()),
                    Some(s) if Self::is_builtin(&s) => unimplemented!("builtin"),
                    Some(_) => call_service(args.collect()),
                    None => Some(IValue::String("error!".to_string())),
                }
            })
        })
    }

    fn is_builtin(_s: &str) -> bool {
        false
    }

    fn into_str(v: &IValue) -> Option<&str> {
        if let IValue::String(s) = v {
            Some(s.as_str())
        } else {
            None
        }
    }

    fn into_string(v: IValue) -> Option<String> {
        if let IValue::String(s) = v {
            Some(s)
        } else {
            None
        }
    }

    pub fn create_service(services: Services, config: ServicesConfig) -> Callback {
        Box::new(move |args| {
            let service_id = uuid::Uuid::new_v4().to_string();
            let make_vm = || {
                let mut args = args.into_iter();
                let blueprint_id = args
                    .next()
                    .and_then(Self::into_string)
                    .ok_or(MissingBlueprintId)?;

                let user_id = args.next().and_then(Self::into_string);
                create_vm(config.clone(), blueprint_id, &service_id, user_id)
            };
            match make_vm() {
                Ok(vm) => {
                    let vm = Arc::new(Mutex::new(vm));
                    services.write().insert(service_id.clone(), vm);
                    Some(IValue::String(service_id))
                }
                // TODO: how to distinguish error from success?
                Err(err) => Some(IValue::String(err.to_string())),
            }
        })
    }

    pub fn call_service(services: Services) -> Callback {
        Box::new(move |args: Vec<IValue>| {
            // TODO: use correct errors instead of MissingBlueprintId
            let parse_args = || -> Result<_, ServiceError> {
                let mut args = args.into_iter();
                let mut next = || -> Result<String, ServiceError> {
                    args.next()
                        .and_then(Self::into_string)
                        .ok_or(MissingBlueprintId)
                };
                let service_id = next()?;
                let fname = next()?;
                let args: serde_json::Value =
                    serde_json::from_str(next()?.as_str()).map_err(|_| MissingBlueprintId)?;
                Ok((service_id, fname, args))
            };
            let call = || -> Result<Vec<IValue>, ServiceError> {
                let (service_id, fname, args) = parse_args()?;
                let services = services.read();
                let vm = services
                    .get(&service_id)
                    .ok_or(ServiceError::NoSuchInstance(service_id))?;
                let result = vm
                    .lock()
                    .call("facade".to_string(), fname, args, <_>::default())
                    .map_err(ServiceError::Engine)?;

                Ok(result)
            };

            match call() {
                Ok(result) => result.into_iter().next(),
                Err(err) => Some(IValue::String(err.to_string())),
            }
        })
    }
}
