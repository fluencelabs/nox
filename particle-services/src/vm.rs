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
use crate::modules::{load_blueprint, load_module_config};
use crate::Result;
use fluence_app_service::vec1::Vec1;
use fluence_app_service::{
    AppService, AppServiceConfig, FaaSConfig, IValue, TomlFaaSConfig, TomlFaaSNamedModuleConfig,
};
use serde_json::json;
use std::path::PathBuf;

pub fn create_vm(
    config: ServicesConfig,
    blueprint_id: String,
    service_id: &str,
    owner_id: Option<String>,
) -> Result<AppService> {
    let to_string = |path: &PathBuf| -> Option<_> { path.to_string_lossy().into_owned().into() };

    // Load configs for all modules in blueprint
    let make_service = move |service_id: &str| -> Result<_> {
        // Load blueprint from disk
        let blueprint = load_blueprint(&config.blueprint_dir, &blueprint_id)?;

        // Load all module configs
        let modules_config: Vec<TomlFaaSNamedModuleConfig> = blueprint
            .dependencies
            .iter()
            .map(|module| load_module_config(&config.modules_dir, module))
            .collect::<Result<_>>()?;

        let modules = AppServiceConfig {
            service_base_dir: config.workdir,
            faas_config: FaaSConfig {
                modules_dir: Some(config.modules_dir),
                modules_config,
                default_modules_config: None,
            },
        };

        let mut envs = config.envs;
        if let Some(owner_id) = owner_id {
            envs.push(format!("owner_id={}", owner_id));
        };

        log::info!("Creating service {}, envs: {:?}", service_id, envs);

        let service = AppService::new(modules, &service_id, envs).map_err(ServiceError::Engine)?;

        // Save created service to disk, so it is recreated on restart
        // persist_service(&config.services_dir, &service_id, &blueprint_id)?;

        Ok(service)
    };

    make_service(service_id)
}

pub fn as_record(v: std::result::Result<IValue, IValue>) -> Option<IValue> {
    let (code, result) = match v {
        Ok(v) => (0, v),
        Err(e) => (1, e),
    };
    let result = ivalue_to_jvalue_string(result);
    let vec = Vec1::new(vec![IValue::U32(code), result]).expect("not empty");
    Some(IValue::Record(vec))
}

/// Serializes IValue to json bytes
fn ivalue_to_jvalue(v: IValue) -> serde_json::Value {
    match v {
        IValue::S8(v) => json!(v),
        IValue::S16(v) => json!(v),
        IValue::S32(v) => json!(v),
        IValue::S64(v) => json!(v),
        IValue::U8(v) => json!(v),
        IValue::U16(v) => json!(v),
        IValue::U32(v) => json!(v),
        IValue::U64(v) => json!(v),
        IValue::F32(v) => json!(v),
        IValue::F64(v) => json!(v),
        IValue::String(v) => json!(v),
        IValue::I32(v) => json!(v),
        IValue::I64(v) => json!(v),
        IValue::Array(v) => json!(v.into_iter().map(ivalue_to_jvalue).collect::<Vec<_>>()),
        IValue::Record(v) => json!(v
            .into_vec()
            .into_iter()
            .map(ivalue_to_jvalue)
            .collect::<Vec<_>>()),
    }
}

#[allow(dead_code)]
fn ivalue_to_jvalue_bytes(ivalue: IValue) -> IValue {
    let jvalue = ivalue_to_jvalue(ivalue);
    let bytes = serde_json::to_vec(&jvalue).expect("shouldn't fail");
    let bytes = bytes.into_iter().map(IValue::U8).collect();
    IValue::Array(bytes)
}

#[allow(dead_code)]
fn ivalue_to_jvalue_string(ivalue: IValue) -> IValue {
    let jvalue = ivalue_to_jvalue(ivalue);
    let string = serde_json::to_string(&jvalue).expect("shouldn't fail");
    IValue::String(string)
}
