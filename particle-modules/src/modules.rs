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

use std::collections::HashSet;
use std::ops::Not;
use std::{collections::HashMap, path::Path, path::PathBuf, sync::Arc};

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use eyre::WrapErr;
use fluence_app_service::{ModuleDescriptor, TomlMarineModuleConfig, TomlMarineNamedModuleConfig};
use fstrings::f;
use marine_it_parser::module_interface;
use marine_module_info_parser::effects;
use marine_module_info_parser::effects::WasmEffect;
use parking_lot::RwLock;
use serde_json::{json, Value as JValue};

use fluence_libp2p::PeerId;
use particle_args::JError;
use particle_execution::{ParticleParams, ParticleVault};
use service_modules::{
    extract_module_file_name, is_blueprint, module_config_name_hash, module_file_name_hash,
    AddBlueprint, Blueprint, Hash,
};

use crate::error::ModuleError::{
    BlueprintNotFound, EmptyDependenciesList, ReadModuleInterfaceError,
};
use crate::error::Result;
use crate::files::{self, load_config_by_path, load_module_descriptor};
use crate::ModuleError::{
    ForbiddenEffector, IncorrectVaultModuleConfig, InvalidEffectorMountedBinary,
    SerializeBlueprintJson,
};

pub type Binaries = HashMap<String, PathBuf>;
pub type Effectors = HashMap<Hash, Binaries>;

#[derive(Debug, Clone)]
pub struct EffectorsMode {
    defined_effectors: Effectors,
    default_binaries: Binaries,
    is_restricted: bool,
}

impl EffectorsMode {
    pub fn restricted_effectors(defined_effectors: Effectors) -> EffectorsMode {
        Self {
            defined_effectors,
            default_binaries: Default::default(),
            is_restricted: true,
        }
    }

    pub fn all_effectors(
        defined_effectors: Effectors,
        default_binaries: Binaries,
    ) -> EffectorsMode {
        Self {
            defined_effectors,
            default_binaries,
            is_restricted: false,
        }
    }
}

impl Default for EffectorsMode {
    fn default() -> Self {
        Self::restricted_effectors(Default::default())
    }
}

#[derive(Debug, Clone)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    module_interface_cache: Arc<RwLock<HashMap<Hash, JValue>>>,
    blueprints: Arc<RwLock<HashMap<String, Blueprint>>>,
    effectors: EffectorsMode,
}

impl ModuleRepository {
    pub fn new(modules_dir: &Path, blueprints_dir: &Path, effectors: EffectorsMode) -> Self {
        let blueprints = Self::load_blueprints(blueprints_dir);
        let blueprints_cache = Arc::new(RwLock::new(blueprints));

        Self {
            modules_dir: modules_dir.to_path_buf(),
            blueprints_dir: blueprints_dir.to_path_buf(),
            module_interface_cache: <_>::default(),
            blueprints: blueprints_cache,
            effectors,
        }
    }

    fn make_effectors_config(
        &self,
        module_name: &str,
        module_hash: &Hash,
        mounted_binaries: HashSet<String>,
    ) -> Result<&HashMap<String, PathBuf>> {
        let binaries = match self.effectors.defined_effectors.get(module_hash) {
            Some(binaries) => Ok(binaries),
            None if !self.effectors.is_restricted => Ok(&self.effectors.default_binaries),
            None => Err(ForbiddenEffector {
                module_name: module_name.to_string(),
                forbidden_cid: module_hash.to_string(),
            }),
        }?;

        for mounted_binary_name in &mounted_binaries {
            if !binaries
                .keys()
                .any(|binary_name| mounted_binary_name == binary_name)
            {
                return Err(InvalidEffectorMountedBinary {
                    module_name: module_name.to_string(),
                    module_cid: module_hash.to_string(),
                    binary_name: mounted_binary_name.clone(),
                });
            }
        }

        Ok(binaries)
    }

    pub fn add_module(&self, name: String, module: Vec<u8>) -> Result<Hash> {
        let hash = Hash::new(&module)?;
        let (logger_enabled, mounted) = Self::get_module_effects(&module)?;
        let effector_settings = mounted
            .is_empty()
            .not()
            .then(|| self.make_effectors_config(&name, &hash, mounted))
            .transpose()?;
        let config = Self::make_config(name, logger_enabled, effector_settings);
        let _config = files::add_module(&self.modules_dir, &hash, &module, config)?;

        Ok(hash)
    }

    // TODO: generate config for modules also
    pub fn add_system_module(
        &self,
        module: Vec<u8>,
        config: TomlMarineNamedModuleConfig,
    ) -> Result<Hash> {
        let hash = Hash::new(&module)?;
        let _config = files::add_module(&self.modules_dir, &hash, &module, config)?;
        Ok(hash)
    }

    pub fn load_module_config_from_vault(
        vault: &ParticleVault,
        // TODO: refactor this out of this crate
        current_peer_id: PeerId,
        config_path: String,
        particle: ParticleParams,
    ) -> Result<TomlMarineNamedModuleConfig> {
        let config = vault.cat_slice(current_peer_id, &particle, Path::new(&config_path))?;
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
        let hash = self.add_module(config.name, module)?;

        Ok(hash.to_string())
    }

    pub fn add_module_from_vault(
        &self,
        vault: &ParticleVault,
        // TODO: refactor this out of this crate
        current_peer_id: PeerId,
        name: String,
        module_path: String,
        particle: ParticleParams,
    ) -> Result<String> {
        let module = vault.cat_slice(current_peer_id, &particle, Path::new(&module_path))?;
        // copy module & config to module_dir
        let hash = self.add_module(name, module)?;

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
                    let blueprint: Blueprint = toml_edit::de::from_slice(&bytes)?;
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

    fn get_module_effects(module: &[u8]) -> Result<(bool, HashSet<String>)> {
        let effects = effects::extract_from_bytes(module)?;
        let mut logger_enabled = false;
        let mut mounted_names = HashSet::new();
        for effect in effects {
            match effect {
                WasmEffect::Logger => {
                    logger_enabled = true;
                }
                WasmEffect::MountedBinary(name) => {
                    mounted_names.insert(name);
                }
            }
        }
        Ok((logger_enabled, mounted_names))
    }

    fn make_config(
        module_name: String,
        logger_enabled: bool,
        effector_settings: Option<&HashMap<String, PathBuf>>,
    ) -> TomlMarineNamedModuleConfig {
        let mounted_binaries = effector_settings.map(|effector_settings| {
            effector_settings
                .iter()
                .map(|(name, path)| (name.clone(), path.to_string_lossy().to_string().into()))
                .collect::<_>()
        });

        TomlMarineNamedModuleConfig {
            name: module_name,
            file_name: None,
            load_from: None,
            config: TomlMarineModuleConfig {
                logger_enabled: Some(logger_enabled),
                wasi: None,
                mounted_binaries,
                logging_mask: None,
            },
        }
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
    use maplit::hashmap;
    use std::assert_matches::assert_matches;
    use std::default::Default;
    use std::path::PathBuf;
    use tempdir::TempDir;

    use service_modules::load_module;
    use service_modules::Hash;

    use crate::ModuleError::{ForbiddenEffector, InvalidEffectorMountedBinary};
    use crate::{AddBlueprint, EffectorsMode, ModuleRepository};

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
        let bp1 = bps1.first().unwrap();
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
    fn test_add_module_effector_allowed() {
        let effector_wasm_cid =
            Hash::from_string("bafkreiepzclggkt57vu7yrhxylfhaafmuogtqly7wel7ozl5k2ehkd44oe")
                .unwrap();

        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::restricted_effectors(hashmap! {
            effector_wasm_cid => hashmap! {
                "ls".to_string() => PathBuf::from("/bin/ls"),
            }
        });

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn test_add_module_effector_forbidden() {
        let some_wasm_cid =
            Hash::from_string("bafkreibjsugno2xsa2ee46xge5t6z4vuwpepyphedbykrfgmm7i6jg6ihe")
                .unwrap();

        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::restricted_effectors(hashmap! {
            some_wasm_cid => hashmap! {
                "ls".to_string() => PathBuf::from("/bin/ls"),
                "cat".to_string() => PathBuf::from("/bin/cat"),
            }
        });

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        assert_matches!(result, Err(ForbiddenEffector { .. }));
    }

    #[test]
    fn test_add_module_effector_invalid() {
        let effector_wasm_cid =
            Hash::from_string("bafkreiepzclggkt57vu7yrhxylfhaafmuogtqly7wel7ozl5k2ehkd44oe")
                .unwrap();

        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::restricted_effectors(hashmap! {
            effector_wasm_cid => hashmap! {
                "cat".to_string() => PathBuf::from("/bin/cat"),
            }
        });

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        let _cat = "cat".to_string();
        assert_matches!(
            result,
            Err(InvalidEffectorMountedBinary {
                binary_name: _cat,
                ..
            })
        );
    }

    // When in dev mode, all effectors are allowed.
    // When an effector is in the list of allowed binaries, the config is taken from the effectors config
    // When an effector isn't in the list, all the binary paths are taken from dev_mode.binaries
    //
    // Here we test that the config for allowed effector is taken from the effectors config
    #[test]
    fn test_add_module_all_effectors_allowed_effector() {
        let effector_wasm_cid =
            Hash::from_string("bafkreiepzclggkt57vu7yrhxylfhaafmuogtqly7wel7ozl5k2ehkd44oe")
                .unwrap();

        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::all_effectors(
            hashmap! {
                effector_wasm_cid => hashmap! {
                    "ls".to_string() => PathBuf::from("/bin/ls"),
                }
            },
            hashmap! { "cat".to_string() => PathBuf::from("/bin/cat")},
        );

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        assert_matches!(result, Ok(_));
    }

    // When in dev mode, all effectors are allowed.
    // When an effector is in the list of allowed binaries, the config is taken from the effectors config
    // When an effector isn't in the list, all the binary paths are taken from dev_mode.binaries
    //
    // Here we test that the config for an undefined effector is taken from the dev_mode.binaries config
    #[test]
    fn test_add_module_all_effectors_undefined_effector() {
        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::all_effectors(
            Default::default(),
            hashmap! { "ls".to_string() => PathBuf::from("/bin/ls")},
        );

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        assert_matches!(result, Ok(_));
    }

    // When in dev mode, all effectors are allowed.
    // When an effector is in the list of allowed binaries, the config is taken from the effectors config
    // When an effector isn't in the list, all the binary paths are taken from dev_mode.binaries
    //
    // Here we test that the config for a defined effector is taken from the effectors config
    // and when the effector config is incorrect, the module creation fails even if the
    // required path is present in the dev_mode.binaries
    //
    // I think this is the least surprising behavior and can prevent errors on deploying/publishing
    #[test]
    fn test_add_module_all_effectors_allowed_effector_wrong_config() {
        let effector_wasm_cid =
            Hash::from_string("bafkreiepzclggkt57vu7yrhxylfhaafmuogtqly7wel7ozl5k2ehkd44oe")
                .unwrap();

        let effector_path = "../crates/nox-tests/tests/effector/artifacts";
        let allowed_effectors = EffectorsMode::all_effectors(
            hashmap! {
                effector_wasm_cid => hashmap! {
                    "cat".to_string() => PathBuf::from("/bin/cat"),
                }
            },
            hashmap! { "ls".to_string() => PathBuf::from("/bin/ls")},
        );

        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test2").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path(), allowed_effectors);

        let module = load_module(effector_path, "effector").expect("load module");
        let result = repo.add_module("effector".to_string(), module);
        let _cat = "cat".to_string();
        assert_matches!(
            result,
            Err(InvalidEffectorMountedBinary {
                binary_name: _cat,
                ..
            })
        );
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

        let result = repo.add_module("pure".to_string(), module);
        assert_matches!(result, Ok(_));
    }
}
