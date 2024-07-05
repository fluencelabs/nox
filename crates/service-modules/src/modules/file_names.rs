/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use super::blueprint::Blueprint;
use cid_utils::Hash;

use std::path::Path;

/// Calculates filename of the config for a wasm module, given a hash or name of the module.
pub fn module_config_name_json(hash: &Hash) -> String {
    format!("{hash}_config.json")
}

/// Calculates the name of a wasm module file, given a hash or name of the module.
pub fn module_file_name(name: &str) -> String {
    format!("{name}.wasm")
}

/// Calculates filename of the config for a wasm module
pub fn module_config_name_hash(config_hash: &Hash) -> String {
    format!("{config_hash}_config.toml")
}

/// Calculates the name of a wasm module file, given a hash of the module.
pub fn module_file_name_hash(module_hash: &Hash) -> String {
    format!("{module_hash}.wasm")
}

/// Calculates filename of the blueprint
pub fn blueprint_file_name(blueprint: &Blueprint) -> String {
    blueprint_fname(blueprint.id.as_str())
}

pub fn blueprint_fname(id: &str) -> String {
    format!("{id}_blueprint.toml")
}

/// Returns true if file is named like a blueprint would be
pub fn is_blueprint(name: &str) -> bool {
    name.ends_with("_blueprint.toml")
}

/// Return file name with .wasm extension stripped. None if extension wasn't .wasm
pub fn extract_module_file_name(path: &Path) -> Option<&str> {
    // return None if extension isn't "wasm"
    path.extension().filter(|ext| ext == &"wasm")?;
    // strip extension
    path.file_stem()?.to_str()
}

pub fn is_module_wasm(path: &Path) -> bool {
    path.extension().map_or(false, |ext| ext == "wasm")
}

pub fn service_file_name(service_id: &str) -> String {
    format!("{service_id}_service.toml")
}

pub fn is_service(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_service.toml"))
}

#[cfg(test)]
mod tests {
    use crate::is_service;
    use crate::{extract_module_file_name, is_blueprint, is_module_wasm};

    use std::path::Path;

    #[test]
    fn is_wasm() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert!(is_module_wasm(path));

        let path = Path::new("/.fluence/services/modules/facade_url_downloader_config.toml");
        assert!(!is_module_wasm(path));
    }

    #[test]
    fn is_bp() {
        let path = "/.fluence/services/modules/facade_url_downloader_blueprint.toml";
        assert!(is_blueprint(path));

        let path = "/.fluence/services/modules/facade_url_downloader.wasm";
        assert!(!is_blueprint(path));
    }

    #[test]
    fn is_srvc() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader_service.toml");
        assert!(is_service(path));

        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert!(!is_service(path));
    }

    #[test]
    fn module_fname() {
        let path = Path::new("/.fluence/services/modules/facade_url_downloader.wasm");
        assert_eq!(
            extract_module_file_name(path),
            Some("facade_url_downloader")
        );

        let path = Path::new("/.fluence/services/modules/facade_url_downloader_config.toml");
        assert_eq!(extract_module_file_name(path), None);
    }
}
