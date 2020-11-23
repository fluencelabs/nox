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

use crate::file_names::extract_module_name;
use crate::{file_names, files, Blueprint};

use host_closure::{closure, closure_opt, Args, Closure};

use serde_json::Value as JValue;
use std::path::PathBuf;

/// Adds a module to the filesystem, overwriting existing module.
pub fn add_module(modules_dir: PathBuf) -> Closure {
    closure_opt(move |mut args| {
        let module: String = Args::next("module", &mut args)?;
        let module = base64::decode(&module).map_err(|err| {
            JValue::String(format!("error decoding module from base64: {:?}", err))
        })?;
        let config = Args::next("config", &mut args)?;
        files::add_module(&modules_dir, module, config)?;

        Ok(None)
    })
}

/// Saves new blueprint to disk
pub fn add_blueprint(blueprint_dir: PathBuf) -> Closure {
    closure(move |mut args| {
        let blueprint = Args::next("blueprint", &mut args)?;
        files::add_blueprint(&blueprint_dir, &blueprint)?;

        Ok(JValue::String(blueprint.id))
    })
}

/// Get available modules (intersection of modules from config + modules on filesystem)
// TODO: load interfaces of these modules
pub fn get_modules(modules_dir: PathBuf) -> Closure {
    closure(move |_| {
        Ok(JValue::Array(
            files::list_files(&modules_dir)
                .into_iter()
                .flatten()
                .filter_map(|pb| extract_module_name(pb.file_name()?.to_str()?).map(JValue::String))
                .collect(),
        ))
    })
}

/// Get available blueprints
pub fn get_blueprints(blueprint_dir: PathBuf) -> Closure {
    closure(move |_| {
        Ok(JValue::Array(
            files::list_files(&blueprint_dir)
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
                        .map_err(|err| log::warn!("failed to read blueprint {}: {:#?}", fname, err))
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
