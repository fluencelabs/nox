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

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    path::PathBuf,
    sync::Arc,
};

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use eyre::WrapErr;
use fluence_app_service::{ModuleDescriptor, TomlMarineNamedModuleConfig};
use fstrings::f;
use marine_it_parser::module_interface;
use marine_module_info_parser::effects;
use marine_module_info_parser::effects::WasmEffect;
use parking_lot::RwLock;
use serde_json::{json, Value as JValue};

use particle_args::JError;
use particle_execution::{ParticleParams, ParticleVault};
use service_modules::{
    extract_module_file_name, is_blueprint, module_config_name_hash, module_file_name_hash,
    AddBlueprint, Blueprint, Hash,
};

use crate::error::ModuleError::{
    BlueprintNotFound, EmptyDependenciesList, ForbiddenMountedBinary, ReadModuleInterfaceError,
};
use crate::error::Result;
use crate::files::{self, load_config_by_path, load_module_descriptor};
use crate::ModuleError::{IncorrectVaultModuleConfig, SerializeBlueprintJson};

#[derive(Debug, Clone)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    module_interface_cache: Arc<RwLock<HashMap<Hash, JValue>>>,
    blueprints: Arc<RwLock<HashMap<String, Blueprint>>>,
    allowed_binaries: HashSet<PathBuf>,
}

impl ModuleRepository {
    pub fn new(
        modules_dir: &Path,
        blueprints_dir: &Path,
        allowed_binaries: HashSet<PathBuf>,
    ) -> Self {
        let blueprints = Self::load_blueprints(blueprints_dir);
        let blueprints_cache = Arc::new(RwLock::new(blueprints));

        Self {
            modules_dir: modules_dir.to_path_buf(),
            blueprints_dir: blueprints_dir.to_path_buf(),
            module_interface_cache: <_>::default(),
            blueprints: blueprints_cache,
            allowed_binaries,
        }
    }

    fn check_module_mounted_binaries(&self, config: &TomlMarineNamedModuleConfig) -> Result<()> {
        if let Some(binaries) = &config.config.mounted_binaries {
            for requested_binary in binaries.values() {
                if let Some(requested_binary) = requested_binary.as_str() {
                    let requested_binary_path = Path::new(requested_binary);
                    if !self.allowed_binaries.contains(requested_binary_path) {
                        return Err(ForbiddenMountedBinary {
                            forbidden_path: requested_binary.to_string(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_module(&self, module: Vec<u8>, config: TomlMarineNamedModuleConfig) -> Result<Hash> {
        let result = effects::extract_from_bytes(&module);
        println!("{:?}", result);

        // TODO: remove unwrap
        let hash = Hash::new(&module).unwrap();

        let config = files::add_module(&self.modules_dir, &hash, &module, config)?;
        self.check_module_mounted_binaries(&config)?;

        Ok(hash)
    }

    pub fn load_module_config_from_vault(
        vault: &ParticleVault,
        config_path: String,
        particle: ParticleParams,
    ) -> Result<TomlMarineNamedModuleConfig> {
        let config = vault.cat_slice(&particle, Path::new(&config_path))?;
        serde_json::from_slice(&config)
            .map_err(|err| IncorrectVaultModuleConfig { config_path, err })
    }

    /// Adds a module to the filesystem, overwriting existing module.
    pub fn add_module_base64(
        &self,
        module: String,
        config: TomlMarineNamedModuleConfig,
    ) -> Result<String> {
        let module = base64.decode(module)?;
        let hash = self.add_module(module, config)?;

        Ok(hash.to_string())
    }

    pub fn add_module_from_vault(
        &self,
        vault: &ParticleVault,
        module_path: String,
        config: TomlMarineNamedModuleConfig,
        particle: ParticleParams,
    ) -> Result<String> {
        let module = vault.cat_slice(&particle, Path::new(&module_path))?;
        // copy module & config to module_dir
        let hash = self.add_module(module, config)?;

        Ok(hash.to_string())
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&self, blueprint: AddBlueprint) -> Result<String> {
        let blueprint_name = blueprint.name.clone();
        if blueprint.dependencies.is_empty() {
            return Err(EmptyDependenciesList { id: blueprint_name });
        }

        let blueprint =
            Blueprint::new(blueprint).map_err(|err| SerializeBlueprintJson(err.to_string()))?;
        files::add_blueprint(&self.blueprints_dir, &blueprint)?;

        self.blueprints
            .write()
            .insert(blueprint.id.clone(), blueprint.clone());

        Ok(blueprint.id)
    }

    pub fn list_modules(&self) -> std::result::Result<JValue, JError> {
        // TODO: refactor errors to enums
        let modules = fs_utils::list_files(&self.modules_dir)
            .into_iter()
            .flatten()
            .filter_map(|path| {
                let hash = extract_module_file_name(&path)?;
                let result: eyre::Result<_> = try {
                    let hash = Hash::from_string(hash).wrap_err(f!("invalid module name {path:?}"))?;
                    let config = self.modules_dir.join(module_config_name_hash(&hash));
                    let config = load_config_by_path(&config).wrap_err(f!("load config ${config:?}"))?;

                    (hash, config)
                };

                let result = match result {
                    Ok((hash, config)) => json!({
                        "name": config.name, 
                        "hash": hash.to_string(),
                        "config": config.config,
                    }),
                    Err(err) => {
                        log::warn!("list_modules error: {:?}", err);
                        json!({
                            "invalid_file_name": hash,
                            "error": format!("{err:?}").split("Stack backtrace:").next().unwrap_or_default(),
                        })
                    }
                };

                Some(result)
            })
            .collect();

        Ok(modules)
    }

    pub fn get_facade_interface(&self, id: &str) -> Result<JValue> {
        let blueprints = self.blueprints.clone();

        let bp = {
            let lock = blueprints.read();
            lock.get(id).cloned()
        };

        match bp {
            None => Err(BlueprintNotFound { id: id.to_string() }),
            Some(bp) => {
                let dep = bp
                    .get_facade_module()
                    .ok_or(EmptyDependenciesList { id: id.to_string() })?;
                self.get_interface_by_hash(&dep)
            }
        }
    }

    pub fn get_interface_by_hash(&self, hash: &Hash) -> Result<JValue> {
        let cache: Arc<RwLock<HashMap<Hash, JValue>>> = self.module_interface_cache.clone();

        get_interface_by_hash(&self.modules_dir, cache, hash)
    }

    pub fn get_interface(&self, hex_hash: &str) -> std::result::Result<JValue, JError> {
        // TODO: refactor errors to ModuleErrors enum
        let interface: eyre::Result<_> = try {
            let hash = Hash::from_string(hex_hash)?;

            get_interface_by_hash(
                &self.modules_dir,
                self.module_interface_cache.clone(),
                &hash,
            )?
        };

        interface.map_err(|err| {
            JError::new(
                format!("{err:?}")
                    // TODO: send patch to eyre so it can be done through their API
                    // Remove backtrace from the response
                    .split("Stack backtrace:")
                    .next()
                    .unwrap_or_default(),
            )
        })
    }

    fn load_blueprints(blueprints_dir: &Path) -> HashMap<String, Blueprint> {
        let blueprints: Vec<Blueprint> = fs_utils::list_files(blueprints_dir)
            .into_iter()
            .flatten()
            .filter_map(|path| {
                // Check if file name matches blueprint schema
                let fname = path.file_name()?.to_str()?;
                if !is_blueprint(fname) {
                    return None;
                }

                let blueprint: eyre::Result<_> = try {
                    // Read & deserialize TOML
                    let bytes = std::fs::read(&path)?;
                    let blueprint: Blueprint = toml::de::from_slice(&bytes)?;
                    blueprint
                };

                match blueprint {
                    Ok(blueprint) => Some(blueprint),
                    Err(err) => {
                        log::warn!("load_blueprints error on file {}: {:?}", fname, err);
                        None
                    }
                }
            })
            .collect();

        let mut bp_map = HashMap::new();
        for bp in blueprints.iter() {
            bp_map.insert(bp.id.clone(), bp.clone());
        }

        bp_map
    }

    pub fn get_blueprint_from_cache(&self, id: &str) -> Result<Blueprint> {
        let blueprints = self.blueprints.clone();
        let blueprints = blueprints.read();
        blueprints
            .get(id)
            .cloned()
            .ok_or(BlueprintNotFound { id: id.to_string() })
    }

    /// Get available blueprints
    pub fn get_blueprints(&self) -> Vec<Blueprint> {
        self.blueprints.read().values().cloned().collect()
    }

    pub fn resolve_blueprint(&self, blueprint_id: &str) -> Result<Vec<ModuleDescriptor>> {
        let blueprint = self.get_blueprint_from_cache(blueprint_id)?;

        // Load all module descriptors
        let module_descriptors: Vec<_> = blueprint
            .dependencies
            .into_iter()
            .map(|m_hash| {
                let config = load_module_descriptor(&self.modules_dir, &m_hash)?;
                Ok(config)
            })
            .collect::<Result<_>>()?;

        Ok(module_descriptors)
    }

    fn check_effectors(&self, module: &[u8]) -> Result<()> {
        let mounted = Self::get_mounted_binaries(module)?;

        Ok(())
    }

    fn get_mounted_binaries(module: &[u8]) -> Result<Vec<String>> {
        let effects = effects::extract_from_bytes(module)?;
        //effects.into_iter().map(|e| e.mounted_binaries).collect()
        let mounted = effects
            .into_iter()
            .filter_map(|e| {
                if let WasmEffect::MountedBinary(bin) = e {
                    Some(bin)
                } else {
                    None
                }
            })
            .collect::<_>();
        Ok(mounted)
    }
}

fn get_interface_by_hash(
    modules_dir: &Path,
    cache: Arc<RwLock<HashMap<Hash, JValue>>>,
    hash: &Hash,
) -> Result<JValue> {
    let interface_cache_opt = {
        let lock = cache.read();
        lock.get(hash).cloned()
    };

    let interface = match interface_cache_opt {
        Some(interface) => interface,
        None => {
            let path = modules_dir.join(module_file_name_hash(hash));
            let interface =
                module_interface(&path).map_err(|err| ReadModuleInterfaceError { path, err })?;
            let json = json!(interface);
            json
        }
    };

    cache.write().insert(hash.clone(), interface.clone());

    Ok(interface)
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_app_service::{TomlMarineModuleConfig, TomlMarineNamedModuleConfig};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use tempdir::TempDir;
    use toml::map::Map;
    use toml::Value::String;

    use service_modules::load_module;
    use service_modules::Hash;

    use crate::{AddBlueprint, ModuleRepository};

    #[test]
    fn test_add_blueprint() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), Default::default());

        let dep1 = Hash::new(&[1, 2, 3]).unwrap();
        let dep2 = Hash::new(&[3, 2, 1]).unwrap();

        let name1 = "bp1".to_string();
        let resp1 = repo
            .add_blueprint(AddBlueprint::new(
                name1.clone(),
                vec![dep1.clone(), dep2.clone()],
            ))
            .unwrap();
        let bps1 = repo.get_blueprints();
        assert_eq!(bps1.len(), 1);
        let bp1 = bps1.get(0).unwrap();
        assert_eq!(bp1.name, name1);

        let name2 = "bp2".to_string();
        let resp2 = repo
            .add_blueprint(AddBlueprint::new("bp2".to_string(), vec![dep1, dep2]))
            .unwrap();
        let bps2 = repo.get_blueprints();
        assert_eq!(bps2.len(), 2);
        let bp2 = bps2.into_iter().find(|bp| bp.name == name2).unwrap();
        assert_eq!(bp2.name, name2);

        assert_ne!(resp1, resp2);
        assert_ne!(bp1.id, bp2.id);
    }

    #[test]
    fn test_add_module_get_interface() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), Default::default());

        let module = load_module(
            "../crates/nox-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: "tetra".to_string(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };

        let m_hash = repo
            .add_module_base64(base64.encode(module), config)
            .unwrap();

        let result = repo.get_interface(&m_hash);
        assert!(result.is_ok())
    }

    #[test]
    fn test_add_module_effector() {
        let _effector_wasm_cid = "bafkreiepzclggkt57vu7yrhxylfhaafmuogtqly7wel7ozl5k2ehkd44oe";
        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let mut mounted = Map::new();
        mounted.insert("ls".to_string(), String("/bin/ls".to_string()));
        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: "effector".to_string(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: None,
                wasi: None,
                mounted_binaries: Some(mounted),
                logging_mask: None,
            },
        };

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let allowed_binaries = HashSet::from([PathBuf::from("/bin/ls")]);
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_binaries);

        let module = load_module(effector_path, &config.name).expect("load module");
        let result = repo.add_module(module, config);
        println!("{:?}", result);
    }

    #[test]
    fn test_add_module_pure() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), Default::default());

        let module = load_module(
            "../crates/nox-tests/tests/tetraplets/artifacts",
            "tetraplets",
        )
        .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: "tetra".to_string(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };
        let m_hash = repo.add_module(module, config).unwrap();
        println!("{:?}", m_hash);
    }
}
