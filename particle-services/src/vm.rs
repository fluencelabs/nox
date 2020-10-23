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
use crate::Result;

use particle_modules::{load_blueprint, load_module_config};

use crate::persistence::persist_service;
use fluence_app_service::{AppService, AppServiceConfig, FaaSConfig};

pub fn create_vm(
    config: ServicesConfig,
    blueprint_id: String,
    service_id: String,
    owner_id: Option<String>,
) -> Result<AppService> {
    // Load configs for all modules in blueprint
    let make_service = move |service_id: String| -> Result<_> {
        // Load blueprint from disk
        let blueprint = load_blueprint(&config.blueprint_dir, &blueprint_id)?;

        // Load all module configs
        let modules_config: Vec<_> = blueprint
            .dependencies
            .iter()
            .map(|module| load_module_config(&config.modules_dir, module).map_err(Into::into))
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
        if let Some(owner_id) = owner_id.clone() {
            envs.insert(b"owner_id".to_vec(), owner_id.into_bytes());
        };

        log::debug!("Creating service {}, envs: {:?}", service_id, envs);

        let service =
            AppService::new(modules, service_id.clone(), envs).map_err(ServiceError::Engine)?;

        // Save created service to disk, so it is recreated on restart
        persist_service(&config.services_dir, service_id, blueprint_id, owner_id)?;

        Ok(service)
    };

    make_service(service_id)
}
