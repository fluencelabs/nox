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

use crate::hash::Hash;
use std::path::Path;

/// Calculates filename of the config for a wasm module
pub(super) fn module_config_name(module_hash: &Hash) -> String {
    format!("{}_config.toml", module_hash.to_hex().as_ref())
}

/// Calculates the name of a wasm module file, given a hash of the module.
pub(super) fn module_file_name(module_hash: &Hash) -> String {
    format!("{}.wasm", module_hash.to_hex().as_ref())
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
pub(super) fn extract_module_file_name(path: &Path) -> Option<&str> {
    // return None if extension isn't "wasm"
    path.extension().filter(|ext| ext == &"wasm")?;
    // strip extension
    path.file_stem()?.to_str()
}

pub(super) fn is_module_wasm(path: &Path) -> bool {
    path.extension().map_or(false, |ext| ext == "wasm")
}

pub fn service_file_name(service_id: &str) -> String {
    format!("{}_service.toml", service_id)
}

pub fn is_service(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_service.toml"))
}

#[cfg(test)]
mod tests {
    use crate::file_names::{extract_module_file_name, is_blueprint, is_module_wasm};
    use crate::is_service;
    use std::path::Path;

    #[test]
    fn is_wasm() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert!(is_module_wasm(&path));

        let path = Path::new("/.fluence/services/modules/facade_url_downloader_config.toml");
        assert!(!is_module_wasm(&path));
    }

    #[test]
    fn is_bp() {
        let path = "/.fluence/services/modules/facade_url_downloader_blueprint.toml";
        assert!(is_blueprint(&path));

        let path = "/.fluence/services/modules/facade_url_downloader.wasm";
        assert!(!is_blueprint(&path));
    }

    #[test]
    fn is_srvc() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader_service.toml");
        assert!(is_service(&path));

        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert!(!is_service(&path));
    }

    #[test]
    fn module_fname() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert_eq!(
            extract_module_file_name(&path),
            Some("facade_url_downloader")
        );

        let path = Path::new("/.fluence/services/modules/facade_url_downloader_config.toml");
        assert_eq!(extract_module_file_name(&path), None);
    }
}
