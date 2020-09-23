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

use fluence_app_service::{AppService, AppServiceError, RawModuleConfig, RawModulesConfig};
use std::task::Waker;
use std::path::PathBuf;
use crate::services::ServiceError;
use crate::config::ServicesConfig;
use crate::Result;
use crate::error::ServiceError;
use crate::modules::{load_blueprint, load_module_config, persist_service};

fn create_app_service(
    config: ServicesConfig,
    blueprint_id: String,
    service_id: String,
    owner_id: Option<String>,
) -> Result<AppService> {
    let to_string =
        |path: &PathBuf| -> Option<_> { path.to_string_lossy().into_owned().into() };

    // Load configs for all modules in blueprint
    let make_service = move |service_id: &str| -> Result<_> {
        // Load blueprint from disk
        let blueprint = load_blueprint(&config.blueprint_dir, &blueprint_id)?;

        // Load all module configs
        let configs: Vec<RawModuleConfig> = blueprint
            .dependencies
            .iter()
            .map(|module| load_module_config(&config.modules_dir, module))
            .collect::<Result<_>>()?;

        let modules = RawModulesConfig {
            modules_dir: to_string(&config.modules_dir),
            service_base_dir: to_string(&config.workdir),
            module: configs,
            default: None,
        };

        let mut envs = config.service_envs;
        if let Some(owner_id) = owner_id {
            envs.push(format!("owner_id={}", owner_id));
        };

        log::info!("Creating service {}, envs: {:?}", service_id, envs);

        let service = AppService::new(modules, &service_id, envs).map_err(ServiceError::AppServiceError)?;

        // Save created service to disk, so it is recreated on restart
        persist_service(&config.services_dir, &service_id, &blueprint_id)?;

        Ok(service)
    };

    let service = make_service(service_id.as_str());

}
