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

use config::{abs_path, create_dirs};
use fluence_app_service::RawModuleConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ActorConfig {
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub envs: Vec<String>,
    /// Working dir for services
    pub workdir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running services
    pub services_dir: PathBuf,
    /// Module config for the stepper
    pub stepper_config: RawModuleConfig,
}

impl ActorConfig {
    pub fn new(
        base_dir: PathBuf,
        envs: Vec<String>,
        stepper_module_name: String,
    ) -> Result<Self, std::io::Error> {
        let base_dir = abs_path(base_dir);

        let mut stepper_config = RawModuleConfig::new(stepper_module_name);
        stepper_config.logger_enabled = Some(true);
        stepper_config.mem_pages_count = Some(100);

        let this = Self {
            workdir: config::workdir(&base_dir),
            modules_dir: config::modules_dir(&base_dir),
            services_dir: config::services_dir(&base_dir),
            envs,
            stepper_config,
        };

        this.create_dirs()?;

        Ok(this)
    }

    pub fn create_dirs(&self) -> Result<(), std::io::Error> {
        create_dirs(&[&self.workdir, &self.modules_dir, &self.services_dir])
    }
}
