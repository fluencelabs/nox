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
use std::path::{Path, PathBuf};

/// Calculates filename of the config for a wasm module
pub(super) fn module_config_name(module_hash: &blake3::Hash) -> String {
    format!("{}_config.toml", module_hash.to_hex())
}

/// Calculates the name of a wasm module file, given a hash of the module.
pub(super) fn module_file_name(module_hash: &blake3::Hash) -> String {
    format!("{}.wasm", module_hash.to_hex())
}

/// Calculates filename of the blueprint
pub(super) fn blueprint_file_name(blueprint: &Blueprint) -> String {
    blueprint_fname(blueprint.id.as_str())
}

pub(super) fn blueprint_fname(id: &str) -> String {
    format!("{}_blueprint.toml", id)
}

/// Returns true if file is named like a blueprint would be
pub(super) fn is_blueprint(name: &str) -> bool {
    name.ends_with("_blueprint.toml")
}

/// Return file name with .wasm extension stripped. None if extension wasn't .wasm
pub(super) fn extract_module_name(name: &str) -> Option<String> {
    let path: &Path = name.as_ref();
    // return None if extension isn't "wasm"
    path.extension().filter(|ext| ext == &"wasm")?;
    // strip extension
    path.file_stem()?.to_string_lossy().to_string().into()
}

pub fn service_file_name(service_id: &str) -> String {
    format!("{}_service.toml", service_id)
}

pub fn is_service(path: &PathBuf) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_service.toml"))
}
