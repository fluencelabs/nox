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

use super::error::ServiceExecError::{self, *};
use super::Blueprint;

use fluence_app_service::{FaaSInterface as AppServiceInterface, RawModuleConfig};

use crate::app_service::error::ServiceExecError::WriteBlueprint;
use crate::app_service::files;
use std::collections::HashMap;
use std::path::PathBuf;

use super::service::*;

impl AppServiceBehaviour {
    /// Get interface of a service specified by `service_id`
    pub fn get_interface(&self, service_id: &str) -> Result<AppServiceInterface<'_>> {
        let service = self
            .app_services
            .get(service_id)
            .ok_or_else(|| ServiceExecError::NoSuchInstance(service_id.to_string()))?;

        Ok(service.get_interface())
    }

    /// Get interfaces for all created services
    pub fn get_interfaces(&self) -> HashMap<&str, AppServiceInterface<'_>> {
        self.app_services
            .iter()
            .map(|(k, v)| (k.as_str(), v.get_interface()))
            .collect()
    }

    /// Get available modules (intersection of modules from config + modules on filesystem)
    // TODO: load interfaces of these modules
    pub fn get_modules(&self) -> Vec<String> {
        Self::list_files(&self.config.modules_dir)
            .into_iter()
            .flatten()
            .filter_map(|pb| files::extract_module_name(pb.file_name()?.to_str()?))
            .collect()
    }

    /// Get available blueprints
    pub fn get_blueprints(&self) -> Vec<Blueprint> {
        Self::list_files(&self.config.blueprint_dir)
            .into_iter()
            .flatten()
            .filter(|pb| {
                pb.file_name()
                    .and_then(|f| f.to_str())
                    .filter(|s| files::is_blueprint(s))
                    .is_some()
            })
            .filter_map(|pb| toml::from_slice(std::fs::read(pb).ok()?.as_slice()).ok())
            .collect()
    }

    /// Adds a module to the filesystem, overwriting existing module.
    /// Also adds module config to the RawModuleConfig
    pub fn add_module(&mut self, bytes: Vec<u8>, config: RawModuleConfig) -> Result<()> {
        let path = PathBuf::from(&self.config.modules_dir);
        let module = path.join(files::module_file_name(&config.name));
        std::fs::write(&module, bytes).map_err(|err| AddModule {
            path: path.clone(),
            err,
        })?;

        // replace existing configuration with a new one
        let toml = toml::to_string_pretty(&config).map_err(|err| SerializeConfig { err })?;
        let config = path.join(files::module_config_name(config.name));
        std::fs::write(&config, toml).map_err(|err| WriteConfig { path, err })?;

        Ok(())
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&mut self, blueprint: &Blueprint) -> Result<()> {
        let mut path = PathBuf::from(&self.config.blueprint_dir);
        path.push(files::blueprint_file_name(&blueprint));

        // Save blueprint to disk
        let bytes = toml::to_vec(&blueprint).map_err(|err| SerializeConfig { err })?;
        std::fs::write(&path, bytes).map_err(|err| WriteBlueprint { path, err })?;

        // TODO: check dependencies are satisfied?

        Ok(())
    }

    /// Load blueprint from disk
    pub(super) fn load_blueprint(bp_dir: &PathBuf, blueprint_id: &str) -> Result<Blueprint> {
        let bp_path = bp_dir.join(files::blueprint_fname(blueprint_id));
        let blueprint =
            std::fs::read(&bp_path).map_err(|err| NoSuchBlueprint { path: bp_path, err })?;
        let blueprint: Blueprint =
            toml::from_slice(blueprint.as_slice()).map_err(|err| IncorrectBlueprint { err })?;

        Ok(blueprint)
    }

    /// Load RawModuleConfig from disk, for a given module name
    pub(super) fn load_module_config(
        modules_dir: &PathBuf,
        module: &str,
    ) -> Result<RawModuleConfig> {
        let config = modules_dir.join(files::module_config_name(module));
        let config = std::fs::read(&config).map_err(|err| NoModuleConfig { path: config, err })?;
        let config =
            toml::from_slice(config.as_slice()).map_err(|err| IncorrectModuleConfig { err })?;

        Ok(config)
    }
}
