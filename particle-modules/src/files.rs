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

use crate::blueprint::Blueprint;
use crate::error::{ModuleError::*, Result};
use crate::file_names;

use fluence_app_service::{FaaSModuleConfig, TomlFaaSNamedModuleConfig};

use std::convert::TryInto;
use std::path::PathBuf;

/// Load blueprint from disk
pub fn load_blueprint(bp_dir: &PathBuf, blueprint_id: &str) -> Result<Blueprint> {
    let bp_path = bp_dir.join(file_names::blueprint_fname(blueprint_id));
    let blueprint =
        std::fs::read(&bp_path).map_err(|err| NoSuchBlueprint { path: bp_path, err })?;
    let blueprint: Blueprint =
        toml::from_slice(blueprint.as_slice()).map_err(|err| IncorrectBlueprint { err })?;

    Ok(blueprint)
}

/// Load FaaSModuleConfig from disk, for a given module name
pub fn load_module_config(
    modules_dir: &PathBuf,
    module: &str,
) -> Result<(String, FaaSModuleConfig)> {
    let config = modules_dir.join(file_names::module_config_name(module));
    let config = std::fs::read(&config).map_err(|err| NoModuleConfig { path: config, err })?;
    let config: TomlFaaSNamedModuleConfig =
        toml::from_slice(config.as_slice()).map_err(|err| IncorrectModuleConfig { err })?;
    let config = config
        .try_into()
        .map_err(|err| ModuleConvertError { err })?;

    Ok(config)
}

/*
/// Persist service info to disk, so it is recreated after restart
pub fn persist_service(services_dir: &PathBuf, service_id: &str, blueprint_id: &str) -> Result<()> {
    let config = PersistedService::new(service_id, blueprint_id);
    let bytes = toml::to_vec(&config).map_err(|err| SerializeConfig { err })?;
    let path = services_dir.join(files::service_file_name(service_id));
    std::fs::write(&path, bytes).map_err(|err| WriteConfig { path, err })
}
*/

/// List files in directory
pub fn list_files(dir: &PathBuf) -> Option<impl Iterator<Item = PathBuf>> {
    let dir = std::fs::read_dir(dir).ok()?;
    Some(dir.filter_map(|p| p.ok()?.path().into()))
}

/// Adds a module to the filesystem, overwriting existing module.
/// Also adds module config to the TomlFaaSNamedModuleConfig
pub fn add_module(
    modules_dir: &PathBuf,
    bytes: Vec<u8>,
    config: TomlFaaSNamedModuleConfig,
) -> Result<()> {
    let module = modules_dir.join(file_names::module_file_name(&config.name));
    std::fs::write(&module, bytes).map_err(|err| AddModule { path: module, err })?;

    // replace existing configuration with a new one
    let toml = toml::to_string_pretty(&config).map_err(|err| SerializeConfig { err })?;
    let config = modules_dir.join(file_names::module_config_name(config.name));
    std::fs::write(&config, toml).map_err(|err| WriteConfig { path: config, err })?;

    Ok(())
}

/// Saves new blueprint to disk
pub fn add_blueprint(blueprint_dir: &PathBuf, blueprint: &Blueprint) -> Result<()> {
    let path = blueprint_dir.join(file_names::blueprint_file_name(&blueprint));

    // Save blueprint to disk
    let bytes = toml::to_vec(&blueprint).map_err(|err| SerializeConfig { err })?;
    std::fs::write(&path, bytes).map_err(|err| WriteBlueprint { path, err })?;

    // TODO: check dependencies are satisfied?

    Ok(())
}
