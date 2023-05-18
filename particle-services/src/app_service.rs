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
use crate::{Result, VIRTUAL_PARTICLE_VAULT_PREFIX};

use fluence_app_service::{
    AppService, AppServiceConfig, MarineConfig, MarineWASIConfig, ModuleDescriptor,
};
use particle_modules::ModuleRepository;
use peer_metrics::ServicesMetrics;
use server_config::ServicesConfig;

use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub fn create_app_service(
    config: ServicesConfig,
    modules: &ModuleRepository,
    blueprint_id: String,
    service_id: String,
    metrics: Option<&ServicesMetrics>,
) -> Result<AppService> {
    let mut modules_config = modules.resolve_blueprint(&blueprint_id)?;
    modules_config
        .iter_mut()
        .for_each(|module| inject_vault(&config.particles_vault_dir, module));

    if let Some(metrics) = metrics.as_ref() {
        metrics.observe_service_config(config.max_heap_size.as_u64(), &modules_config);
    }

    let app_config = AppServiceConfig {
        service_working_dir: config.workdir.join(&service_id),
        service_base_dir: config.workdir,
        marine_config: MarineConfig {
            modules_dir: Some(config.modules_dir),
            modules_config,
            default_modules_config: None,
        },
    };

    log::debug!("Creating service {}, envs: {:?}", service_id, config.envs);
    AppService::new(app_config, service_id, config.envs).map_err(ServiceError::Engine)
}

/// Map `vault_dir` to `/tmp/vault` inside the service.
/// Particle File Vaults will be available as `/tmp/vault/$particle_id`
fn inject_vault(vault_dir: &Path, module: &mut ModuleDescriptor) {
    let wasi = &mut module.config.wasi;
    if wasi.is_none() {
        *wasi = Some(MarineWASIConfig::default());
    }
    // SAFETY: set wasi to Some in the code above
    let wasi = wasi.as_mut().unwrap();

    let vault_dir = vault_dir.to_path_buf();

    wasi.preopened_files.insert(vault_dir.clone());
    wasi.mapped_dirs
        .insert(VIRTUAL_PARTICLE_VAULT_PREFIX.into(), vault_dir);
}
