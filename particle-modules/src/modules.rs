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

use bytesize::ByteSize;
use std::{collections::HashMap, iter, path::Path, path::PathBuf, sync::Arc};

use eyre::WrapErr;
use fluence_app_service::{ModuleDescriptor, TomlMarineNamedModuleConfig};
use fstrings::f;
use marine_it_parser::module_interface;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JValue};

use fs_utils::file_name;
use particle_args::JError;
use particle_execution::ParticleParams;
use service_modules::{
    extract_module_file_name, hash_dependencies, is_blueprint, is_module_wasm,
    module_config_name_hash, module_file_name_hash, Blueprint, Dependency, Hash,
};

use crate::error::ModuleError::{
    BlueprintNotFound, BlueprintNotFoundInVault, ConfigNotFoundInVault, EmptyDependenciesList,
    FacadeShouldBeHash, IncorrectVaultBlueprint, IncorrectVaultModuleConfig, InvalidBlueprintPath,
    InvalidModuleConfigPath, InvalidModuleName, InvalidModulePath, MaxHeapSizeOverflow,
    ModuleNotFoundInVault, ReadModuleInterfaceError, VaultDoesNotExist,
};
use crate::error::Result;
use crate::files::{self, load_config_by_path, load_module_by_path, load_module_descriptor};

type ModuleName = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddBlueprint {
    pub name: String,
    pub dependencies: Vec<Dependency>,
}

impl AddBlueprint {
    pub fn new(name: String, dependencies: Vec<Dependency>) -> Self {
        Self { name, dependencies }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    particles_vault_dir: PathBuf,
    /// Map of module_config.name to blake3::hash(module bytes)
    modules_by_name: Arc<Mutex<HashMap<ModuleName, Hash>>>,
    module_interface_cache: Arc<RwLock<HashMap<Hash, JValue>>>,
    blueprints: Arc<RwLock<HashMap<String, Blueprint>>>,
    max_heap_size: ByteSize,
    default_heap_size: Option<ByteSize>,
}

impl ModuleRepository {
    pub fn new(
        modules_dir: &Path,
        blueprints_dir: &Path,
        particles_vault_dir: &Path,
        max_heap_size: ByteSize,
        default_heap_size: Option<ByteSize>,
    ) -> Self {
        let modules_by_name: HashMap<_, _> = files::list_files(modules_dir)
            .into_iter()
            .flatten()
            .filter(|path| is_module_wasm(path))
            .filter_map(|path| {
                let name_hash: Result<_> = try {
                    let module = load_module_by_path(&path)?;
                    let hash = Hash::hash(&module);

                    Self::maybe_migrate_module(&path, &hash, modules_dir);

                    let module = load_module_descriptor(modules_dir, &hash)?;
                    (module.import_name, hash)
                };

                match name_hash {
                    Ok(name_hash) => Some(name_hash),
                    Err(err) => {
                        log::warn!("Error loading module list: {:?}", err);
                        None
                    }
                }
            })
            .collect();

        let modules_by_name = Arc::new(Mutex::new(modules_by_name));

        let blueprints = Self::load_blueprints(blueprints_dir);
        let blueprints_cache = Arc::new(RwLock::new(blueprints));

        Self {
            modules_by_name,
            modules_dir: modules_dir.to_path_buf(),
            blueprints_dir: blueprints_dir.to_path_buf(),
            module_interface_cache: <_>::default(),
            blueprints: blueprints_cache,
            particles_vault_dir: particles_vault_dir.to_path_buf(),
            max_heap_size,
            default_heap_size,
        }
    }

    /// check that module file name is equal to module hash
    /// if not, rename module and config files
    fn maybe_migrate_module(path: &Path, hash: &Hash, modules_dir: &Path) {
        use eyre::eyre;

        let migrated: eyre::Result<_> = try {
            let file_name = extract_module_file_name(path).ok_or_else(|| eyre!("no file name"))?;
            if file_name != hash.to_hex().as_ref() {
                let new_name = module_file_name_hash(hash);
                log::debug!(target: "migration", "renaming module {}.wasm to {}", file_name, new_name);
                std::fs::rename(&path, modules_dir.join(module_file_name_hash(hash)))?;
                let new_name = module_config_name_hash(hash);
                let config = path.with_file_name(format!("{}_config.toml", file_name));
                log::debug!(target: "migration", "renaming config {:?} to {}", config.file_name().unwrap(), new_name);
                std::fs::rename(&config, modules_dir.join(new_name))?;
            }
        };

        if let Err(e) = migrated {
            log::warn!("Module {:?} migration failed: {:?}", path, e);
        }
    }

    // set default if max_heap_size and mem_pages_count are not specified
    fn check_module_heap_size(&self, config: &mut TomlMarineNamedModuleConfig) -> Result<()> {
        let heap_size = match (config.config.max_heap_size, config.config.mem_pages_count) {
            (Some(heap_size), _) => Some(heap_size),
            (None, Some(pages_count)) => {
                Some(ByteSize::b(marine_utils::wasm_pages_to_bytes(pages_count)))
            }
            (None, None) => self.default_heap_size,
        };

        config.config.max_heap_size = heap_size;

        if let Some(heap_size) = heap_size {
            if heap_size > self.max_heap_size {
                return Err(MaxHeapSizeOverflow {
                    max_heap_size_wanted: heap_size.as_u64(),
                    max_heap_size_allowed: self.max_heap_size.as_u64(),
                });
            }
        }

        Ok(())
    }
    fn add_module(&self, module: Vec<u8>, config: TomlMarineNamedModuleConfig) -> Result<String> {
        let hash = Hash::hash(&module);

        let mut config = files::add_module(&self.modules_dir, &hash, &module, config)?;
        self.check_module_heap_size(&mut config)?;
        let module_hash = hash.to_hex().as_ref().to_owned();
        self.modules_by_name.lock().insert(config.name, hash);

        Ok(module_hash)
    }

    fn check_vault_exists(&self, particle_id: &str) -> Result<PathBuf> {
        let vault_path = self.particles_vault_dir.join(particle_id);
        if !vault_path.exists() {
            return Err(VaultDoesNotExist { vault_path });
        }
        Ok(vault_path)
    }

    pub fn load_module_config_from_vault(
        &self,
        config_path: String,
        particle: ParticleParams,
    ) -> Result<TomlMarineNamedModuleConfig> {
        let vault_path = self.check_vault_exists(&particle.id)?;
        // load & deserialize module config from vault
        let config_fname =
            file_name(&config_path).map_err(|err| InvalidModuleConfigPath { err, config_path })?;
        let config_path = vault_path.join(config_fname);
        let config = std::fs::read(&config_path).map_err(|err| {
            let config_path = config_path.clone();
            ConfigNotFoundInVault { config_path, err }
        })?;

        serde_json::from_slice(&config)
            .map_err(|err| IncorrectVaultModuleConfig { config_path, err })
    }

    pub fn load_blueprint_from_vault(
        &self,
        blueprint_path: String,
        particle: ParticleParams,
    ) -> Result<AddBlueprint> {
        let vault_path = self.check_vault_exists(&particle.id)?;

        // load & deserialize module config from vault
        let blueprint_fname = file_name(&blueprint_path).map_err(|err| InvalidBlueprintPath {
            err,
            blueprint_path,
        })?;
        let blueprint_path = vault_path.join(blueprint_fname);
        let blueprint = std::fs::read(&blueprint_path).map_err(|err| {
            let blueprint_path = blueprint_path.clone();
            BlueprintNotFoundInVault {
                blueprint_path,
                err,
            }
        })?;

        serde_json::from_slice(&blueprint).map_err(|err| IncorrectVaultBlueprint {
            blueprint_path,
            err,
        })
    }

    /// Adds a module to the filesystem, overwriting existing module.
    pub fn add_module_base64(
        &self,
        module: String,
        config: TomlMarineNamedModuleConfig,
    ) -> Result<String> {
        let module = base64::decode(&module)?;
        self.add_module(module, config)
    }

    pub fn add_module_from_vault(
        &self,
        module_path: String,
        config: TomlMarineNamedModuleConfig,
        particle: ParticleParams,
    ) -> Result<String> {
        let vault_path = self.check_vault_exists(&particle.id)?;

        // load module
        let module_fname =
            file_name(&module_path).map_err(|err| InvalidModulePath { err, module_path })?;
        let module_path = vault_path.join(module_fname);
        let module = std::fs::read(&module_path)
            .map_err(|err| ModuleNotFoundInVault { module_path, err })?;

        // copy module & config to module_dir
        self.add_module(module, config)
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&self, blueprint: AddBlueprint) -> Result<String> {
        // resolve dependencies by name to hashes, if any
        let mut dependencies: Vec<Hash> = blueprint
            .dependencies
            .into_iter()
            .map(|module| resolve_hash(&self.modules_by_name, module))
            .collect::<Result<_>>()?;

        let blueprint_name = blueprint.name.clone();
        let facade = dependencies
            .pop()
            .ok_or(EmptyDependenciesList { id: blueprint_name })?;

        let hash = hash_dependencies(facade.clone(), dependencies.clone()).to_hex();

        let blueprint = Blueprint {
            id: hash.as_ref().to_string(),
            dependencies: dependencies
                .into_iter()
                .map(Dependency::Hash)
                .chain(iter::once(Dependency::Hash(facade)))
                .collect(),
            name: blueprint.name,
        };
        files::add_blueprint(&self.blueprints_dir, &blueprint)?;

        self.blueprints
            .write()
            .insert(blueprint.id.clone(), blueprint.clone());

        Ok(blueprint.id)
    }

    pub fn list_modules(&self) -> std::result::Result<JValue, JError> {
        // TODO: refactor errors to enums
        let modules = files::list_files(&self.modules_dir)
            .into_iter()
            .flatten()
            .filter_map(|path| {
                let hash = extract_module_file_name(&path)?;
                let result: eyre::Result<_> = try {
                    let hash = Hash::from_hex(hash).wrap_err(f!("invalid module name {path:?}"))?;
                    let config = self.modules_dir.join(module_config_name_hash(&hash));
                    let config = load_config_by_path(&config).wrap_err(f!("load config ${config:?}"))?;

                    (hash, config)
                };

                let result = match result {
                    Ok((hash, config)) => json!({
                        "name": config.name,
                        "hash": hash.to_hex().as_ref(),
                        "config": config.config,
                    }),
                    Err(err) => {
                        log::warn!("list_modules error: {:?}", err);
                        json!({
                            "invalid_file_name": hash,
                            "error": format!("{:?}", err).split("Stack backtrace:").next().unwrap_or_default(),
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

                let hash = match dep {
                    Dependency::Hash(hash) => hash,
                    Dependency::Name(_) => return Err(FacadeShouldBeHash { id: id.to_string() }),
                };

                self.get_interface_by_hash(&hash)
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
            let hash = Hash::from_hex(hex_hash)?;

            get_interface_by_hash(
                &self.modules_dir,
                self.module_interface_cache.clone(),
                &hash,
            )?
        };

        interface.map_err(|err| {
            JError::new(
                format!("{:?}", err)
                    // TODO: send patch to eyre so it can be done through their API
                    // Remove backtrace from the response
                    .split("Stack backtrace:")
                    .next()
                    .unwrap_or_default(),
            )
        })
    }

    fn load_blueprints(blueprints_dir: &Path) -> HashMap<String, Blueprint> {
        let blueprints: Vec<Blueprint> = files::list_files(blueprints_dir)
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
                    let blueprint: Blueprint = toml::from_slice(&bytes)?;
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

    fn get_blueprint_from_cache(&self, id: &str) -> Result<Blueprint> {
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
            .map(|module| {
                let hash = resolve_hash(&self.modules_by_name, module)?;
                let config = load_module_descriptor(&self.modules_dir, &hash)?;
                Ok(config)
            })
            .collect::<Result<_>>()?;

        Ok(module_descriptors)
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

fn resolve_hash(
    modules: &Arc<Mutex<HashMap<ModuleName, Hash>>>,
    module: Dependency,
) -> Result<Hash> {
    match module {
        Dependency::Hash(hash) => Ok(hash),
        Dependency::Name(name) => {
            // resolve module hash by name
            let map = modules.lock();
            let hash = map.get(&name).cloned();
            hash.ok_or_else(|| InvalidModuleName(name.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use bytesize::ByteSize;
    use fluence_app_service::{TomlMarineModuleConfig, TomlMarineNamedModuleConfig};
    use std::str::FromStr;
    use tempdir::TempDir;

    use service_modules::load_module;
    use service_modules::{Dependency, Hash};

    use crate::error::ModuleError::MaxHeapSizeOverflow;
    use crate::{AddBlueprint, ModuleRepository};

    #[test]
    fn test_add_blueprint() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test").unwrap();
        let vault_dir = TempDir::new("test").unwrap();
        let max_heap_size = server_config::default_module_max_heap_size();
        let repo = ModuleRepository::new(
            module_dir.path(),
            bp_dir.path(),
            vault_dir.path(),
            max_heap_size,
            None,
        );

        let dep1 = Dependency::Hash(Hash::hash(&[1, 2, 3]));
        let dep2 = Dependency::Hash(Hash::hash(&[3, 2, 1]));

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
        assert_eq!(bps2.len(), 1);
        let bp2 = bps2.get(0).unwrap();
        assert_eq!(bp2.name, name2);

        assert_eq!(resp1, resp2);
        assert_eq!(bp1.id, bp2.id);
    }

    #[test]
    fn test_add_module_get_interface() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let vault_dir = TempDir::new("test3").unwrap();
        let max_heap_size = server_config::default_module_max_heap_size();
        let repo = ModuleRepository::new(
            module_dir.path(),
            bp_dir.path(),
            vault_dir.path(),
            max_heap_size,
            None,
        );

        let module = load_module("../particle-node/tests/tetraplets/artifacts", "tetraplets")
            .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: "tetra".to_string(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                mem_pages_count: None,
                max_heap_size: None,
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };

        let hash = repo
            .add_module_base64(base64::encode(module), config)
            .unwrap();

        let result = repo.get_interface(&hash);
        assert!(result.is_ok())
    }

    #[test]
    fn test_hash_dependency() {
        use super::hash_dependencies;
        use crate::modules::Hash;

        let dep1 = Hash::hash(&[1, 2, 3]);
        let dep2 = Hash::hash(&[2, 1, 3]);
        let dep3 = Hash::hash(&[3, 2, 1]);

        let hash1 = hash_dependencies(dep3.clone(), vec![dep1.clone(), dep2.clone()]);
        let hash2 = hash_dependencies(dep3.clone(), vec![dep2.clone(), dep1.clone()]);
        let hash3 = hash_dependencies(dep1.clone(), vec![dep2.clone(), dep3.clone()]);
        assert_eq!(hash1.to_string(), hash2.to_string());
        assert_ne!(hash2.to_string(), hash3.to_string());
    }

    #[test]
    fn test_add_module_max_heap_size_overflow() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let vault_dir = TempDir::new("test3").unwrap();
        let max_heap_size = ByteSize::from_str("10 Mb").unwrap();
        let repo = ModuleRepository::new(
            module_dir.path(),
            bp_dir.path(),
            vault_dir.path(),
            max_heap_size,
            None,
        );

        let module = load_module("../particle-node/tests/tetraplets/artifacts", "tetraplets")
            .expect("load module");

        let config: TomlMarineNamedModuleConfig = TomlMarineNamedModuleConfig {
            name: "tetra".to_string(),
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                mem_pages_count: None,
                max_heap_size: Some(ByteSize::b(max_heap_size.as_u64() + 10)),
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };

        let result = repo.add_module_base64(base64::encode(module), config);

        assert!(result.is_err());
        assert_eq!(
            format!("{:?}", result.unwrap_err()),
            format!(
                "{:?}",
                MaxHeapSizeOverflow {
                    max_heap_size_wanted: max_heap_size.as_u64() + 10,
                    max_heap_size_allowed: max_heap_size.as_u64()
                }
            )
        );
    }
}
