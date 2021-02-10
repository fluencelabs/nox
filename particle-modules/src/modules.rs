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

use crate::dependency::Dependency;
use crate::file_names::{extract_module_name, is_module_wasm};
use crate::{file_names, files, load_module_config, Blueprint, ModuleError};

use fce_wit_parser::module_interface;
use host_closure::{closure, closure_opt, Args, Closure};

use crate::files::load_module_by_path;
use parking_lot::Mutex;
use serde_json::{json, Value as JValue};
use std::path::Path;
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

type ModuleName = String;
#[allow(dead_code)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    /// Map of module_config.name to Dependency::Hash
    modules_by_name: Arc<Mutex<HashMap<ModuleName, Dependency>>>,
}

impl ModuleRepository {
    pub fn new(modules_dir: &Path, blueprints_dir: &Path) -> Self {
        let modules_by_name: HashMap<_, _> = files::list_files(&modules_dir)
            .into_iter()
            .flatten()
            .filter(|path| is_module_wasm(&path))
            .filter_map(|path| {
                let name_hash: Result<_, ModuleError> = try {
                    let module = load_module_by_path(&path)?;
                    let hash = Dependency::hash(&module);
                    let module = load_module_config(&modules_dir, &hash)?;
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
            let hash = Dependency::hash(&module);
            let config = Args::next("config", &mut args)?;
            files::add_module(&modules_dir, &hash, &module, &config)?;

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
                    let fname = extract_module_name(&path)?;
                    let interface = module_interface(&path)
                        .map_err(|err| {
                            log::warn!(
                                "Failed to retrieve interface for module {}: {:#?}",
                                fname,
                                err
                            )
                        })
                        .ok()?;
                    let interface = serde_json::to_value(interface).map_err(|err| {
                        log::warn!(
                            "Failed to serialize interface for module {}: {:#?}",
                            fname,
                            err
                        );
                        JValue::String(format!("Corrupted interface: {:?}", err))
                    });
                    let interface = match interface {
                        Ok(value) => value,
                        Err(value) => value,
                    };

                    Some(json!({
                        "name": fname,
                        "interface": interface
                    }))
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
}
