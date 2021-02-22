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

use crate::error::ServiceError;
use crate::persistence::{persist_service, PersistedService};
use crate::Result;

use particle_modules::ModuleRepository;
use server_config::ServicesConfig;

use fluence_app_service::{AppService, AppServiceConfig, FaaSConfig};

pub fn create_app_service(
    config: ServicesConfig,
    modules: &ModuleRepository,
    blueprint_id: String,
    service_id: String,
    aliases: Vec<String>,
    owner_id: String,
) -> Result<AppService> {
    try {
        let modules_config = modules.resolve_blueprint(&blueprint_id)?;

        let modules = AppServiceConfig {
            service_base_dir: config.workdir,
            faas_config: FaaSConfig {
                modules_dir: Some(config.modules_dir),
                modules_config,
                default_modules_config: None,
            },
        };

        log::debug!("Creating service {}, envs: {:?}", service_id, config.envs);

        let service = AppService::new(modules, service_id.clone(), config.envs)
            .map_err(ServiceError::Engine)?;

        // Save created service to disk, so it is recreated on restart
        let persisted = PersistedService::new(service_id.clone(), blueprint_id, aliases, owner_id);
        persist_service(&config.services_dir, persisted)?;

        service
    }
}
