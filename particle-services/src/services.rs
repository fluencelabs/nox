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
use crate::vm::{as_record, create_vm};

use aquamarine_vm::HostImportDescriptor;
use fluence_app_service::{AppService, IType, IValue};

use crate::args::{parse_args, Args};
use parking_lot::{Mutex, RwLock};
use std::{collections::HashMap, sync::Arc};

type Fabric = Arc<dyn Fn() -> HostImportDescriptor + Send + Sync + 'static>;

type VM = Arc<Mutex<AppService>>;
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

    pub fn fabric(&self) -> Fabric {
        let services = self.services.clone();
        let config = self.config.clone();

        Arc::new(move || {
            let services = services.clone();
            let config = config.clone();
            let closure = Box::new(move |args| {
                let services = services.clone();
                let config = config.clone();
                let args = match parse_args(args) {
                    Ok(args) => args,
                    Err(err) => {
                        log::warn!("error parsing args: {:?}", err);
                        return as_record(Err(IValue::String(err.to_string())));
                    }
                };
                log::info!("calling host with args: {:?}", args);
                // route
                match args.service_id.as_str() {
                    "create" => Self::create_service(services, config, args.args),
                    s if Self::is_builtin(&s) => unimplemented!("builtin"),
                    _ => Self::call_service(services, args),
                }
            });
            HostImportDescriptor {
                closure,
                argument_types: vec![IType::String, IType::String, IType::String],
                output_type: Some(IType::Record(0)),
                error_handler: None,
            }
        })
    }

    fn is_builtin(_s: &str) -> bool {
        false
    }

    pub fn create_service(
        services: Services,
        config: ServicesConfig,
        args: serde_json::Value,
    ) -> Option<IValue> {
        let service_id = uuid::Uuid::new_v4().to_string();
        let make_vm = || {
            let blueprint_id = args
                .get("blueprint_id")
                .and_then(|v| v.as_str())
                .ok_or(MissingBlueprintId)?
                .to_string();

            let user_id = args
                .get("user_id")
                .and_then(|v| Some(v.as_str()?.to_string()));
            create_vm(config.clone(), blueprint_id, &service_id, user_id)
        };
        let result = match make_vm() {
            Ok(vm) => {
                let vm = Arc::new(Mutex::new(vm));
                services.write().insert(service_id.clone(), vm);
                Ok(IValue::String(service_id))
            }
            // TODO: how to distinguish error from success?
            Err(err) => Err(IValue::String(err.to_string())),
        };
        as_record(result)
    }

    pub fn call_service(services: Services, args: Args) -> Option<IValue> {
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

        let result = match call() {
            // AppService always returns a single element
            Ok(result) => Ok(result.into_iter().next().expect("must be defined")),
            Err(err) => Err(IValue::String(err.to_string())),
        };

        as_record(result)
    }
}
