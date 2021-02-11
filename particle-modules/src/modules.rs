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

use crate::dependency::{Dependency, ModuleHash};
use crate::error::ModuleError::InvalidModuleName;
use crate::error::Result;
use crate::file_names::{extract_module_name, is_module_wasm};
use crate::files::{load_config_by_path, load_module_by_path};
use crate::{file_names, files, load_blueprint, load_module_descriptor, Blueprint};

use fce_wit_parser::module_interface;
use fluence_app_service::ModuleDescriptor;
use host_closure::{closure, closure_opt, Args, Closure};

use eyre::WrapErr;
use parking_lot::Mutex;
use serde_json::{json, Value as JValue};
use std::{collections::HashMap, path::Path, path::PathBuf, sync::Arc};

type ModuleName = String;
#[derive(Clone)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    /// Map of module_config.name to blake3::hash(module bytes)
    modules_by_name: Arc<Mutex<HashMap<ModuleName, ModuleHash>>>,
}

impl ModuleRepository {
    pub fn new(modules_dir: &Path, blueprints_dir: &Path) -> Self {
        let modules_by_name: HashMap<_, _> = files::list_files(&modules_dir)
            .into_iter()
            .flatten()
            .filter(|path| is_module_wasm(&path))
            .filter_map(|path| {
                let name_hash: Result<_> = try {
                    let module = load_module_by_path(&path)?;
                    let hash = ModuleHash::hash(&module);
                    let module = load_module_descriptor(&modules_dir, &hash)?;
                    (module.import_name, hash)
                };

                match name_hash {
                    Ok(name_hash) => Some(name_hash),
                    Err(err) => {
                        log::warn!("Error loading module list: {}", err);
                        None
                    }
                }
            })
            .collect();

        let modules_by_name = Arc::new(Mutex::new(modules_by_name));

        Self {
            modules_by_name,
            modules_dir: modules_dir.to_path_buf(),
            blueprints_dir: blueprints_dir.to_path_buf(),
        }
    }

    /// Adds a module to the filesystem, overwriting existing module.
    pub fn add_module(&self) -> Closure {
        let modules = self.modules_by_name.clone();
        let modules_dir = self.modules_dir.clone();
        closure_opt(move |mut args| {
            let module: String = Args::next("module", &mut args)?;
            let module = base64::decode(&module).map_err(|err| {
                JValue::String(format!("error decoding module from base64: {:?}", err))
            })?;
            let hash = ModuleHash::hash(&module);
            let config = Args::next("config", &mut args)?;
            let config = files::add_module(&modules_dir, &hash, &module, config)?;

            modules.lock().insert(config.name, hash);

            Ok(None)
        })
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&self) -> Closure {
        let blueprints_dir = self.blueprints_dir.clone();
        closure(move |mut args| {
            let blueprint = Args::next("blueprint", &mut args)?;
            files::add_blueprint(&blueprints_dir, &blueprint)?;

            Ok(JValue::String(blueprint.id))
        })
    }

    /// Get available modules (intersection of modules from config + modules on filesystem)
    pub fn get_modules(&self) -> Closure {
        let modules_dir = self.modules_dir.clone();
        closure(move |_| {
            let modules = files::list_files(&modules_dir)
                .into_iter()
                .flatten()
                .filter_map(|path| {
                    let hash = extract_module_name(&path)?;
                    let hash = ModuleHash::from_hex(hash);
                    let config = modules_dir.join(hash.config_file_name());
                    let result: eyre::Result<_> = try {
                        let config = load_config_by_path(&config).wrap_err("load config")?;
                        let interface = module_interface(&path).wrap_err("parse interface")?;
                        let interface = serde_json::to_value(interface).wrap_err("serialize")?;
                        (config, interface)
                    };
                    let result = match result {
                        Ok((config, interface)) => json!({
                            "name": config.name,
                            "hash": hash.to_hex().as_ref(),
                            "interface": interface,
                            "config": config.config,
                        }),
                        Err(err) => {
                            log::warn!("get_modules error: {:?}", err);
                            json!({
                                "hash": hash.to_hex().as_ref(),
                                "error": format!("{:?}", err).split("Stack backtrace:").next().unwrap_or_default(),
                            })
                        }
                    };

                    Some(result)
                })
                .collect();

            Ok(JValue::Array(modules))
        })
    }

    /// Get available blueprints
    pub fn get_blueprints(&self) -> Closure {
        let blueprints_dir = self.blueprints_dir.clone();

        closure(move |_| {
            Ok(JValue::Array(
                files::list_files(&blueprints_dir)
                    .into_iter()
                    .flatten()
                    .filter_map(|pb| {
                        // Check if file name matches blueprint schema
                        let fname = pb
                            .file_name()?
                            .to_str()
                            .filter(|s| file_names::is_blueprint(s))?;

                        // Read & deserialize TOML
                        let bytes = std::fs::read(&pb)
                            .map_err(|err| {
                                log::warn!("failed to read blueprint {}: {:#?}", fname, err)
                            })
                            .ok()?;
                        let blueprint: Blueprint = toml::from_slice(bytes.as_slice())
                            .map_err(|err| {
                                log::warn!("failed to deserialize blueprint {}: {:#?}", fname, err)
                            })
                            .ok()?;

                        // Convert to json
                        serde_json::to_value(blueprint).ok()
                    })
                    .collect(),
            ))
        })
    }

    pub fn resolve_blueprint(&self, blueprint_id: &str) -> Result<Vec<ModuleDescriptor>> {
        let blueprint = load_blueprint(&self.blueprints_dir, blueprint_id)?;

        // Load all module descriptors
        let module_descriptors: Vec<_> = blueprint
            .dependencies
            .iter()
            .map(|module| {
                let hash = match module {
                    Dependency::Hash(hash) => hash.clone(),
                    Dependency::Name(name) => {
                        // resolve module hash by name
                        let map = self.modules_by_name.lock();
                        let hash = map.get(name).cloned();
                        // unlock mutex
                        drop(map);
                        hash.ok_or_else(|| InvalidModuleName(name.clone()))?
                    }
                };
                let config = load_module_descriptor(&self.modules_dir, &hash)?;
                Ok(config)
            })
            .collect::<Result<_>>()?;

        Ok(module_descriptors)
    }
}
