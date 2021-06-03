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
use crate::error::ModuleError::{
    BlueprintNotFound, EmptyDependenciesList, FacadeShouldBeHash, InvalidModuleName,
    InvalidModuleReference, ReadModuleInterfaceError,
};
use crate::error::Result;
use crate::file_names::{extract_module_file_name, is_module_wasm};
use crate::file_names::{module_config_name, module_file_name};
use crate::files::{load_config_by_path, load_module_by_path};
use crate::hash::Hash;
use crate::{file_names, files, load_module_descriptor, Blueprint};

use fluence_app_service::ModuleDescriptor;
use host_closure::{closure, Args, Closure};
use marine_it_parser::module_interface;

use eyre::WrapErr;
use fstrings::f;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JValue};
use std::{collections::HashMap, path::Path, path::PathBuf, sync::Arc};

type ModuleName = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddBlueprint {
    pub name: String,
    pub dependencies: Vec<Dependency>,
}

#[derive(Clone)]
pub struct ModuleRepository {
    modules_dir: PathBuf,
    blueprints_dir: PathBuf,
    /// Map of module_config.name to blake3::hash(module bytes)
    modules_by_name: Arc<Mutex<HashMap<ModuleName, Hash>>>,
    module_interface_cache: Arc<RwLock<HashMap<Hash, JValue>>>,
    blueprints: Arc<RwLock<HashMap<String, Blueprint>>>,
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
                    let hash = Hash::hash(&module);

                    Self::maybe_migrate_module(&path, &hash, &modules_dir);

                    let module = load_module_descriptor(&modules_dir, &hash)?;
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
        }
    }

    /// check that module file name is equal to module hash
    /// if not, rename module and config files
    fn maybe_migrate_module(path: &Path, hash: &Hash, modules_dir: &Path) {
        use eyre::eyre;

        let migrated: eyre::Result<_> = try {
            let file_name = extract_module_file_name(&path).ok_or_else(|| eyre!("no file name"))?;
            if file_name != hash.to_hex().as_ref() {
                let new_name = module_file_name(hash);
                log::debug!(target: "migration", "renaming module {}.wasm to {}", file_name, new_name);
                std::fs::rename(&path, modules_dir.join(module_file_name(hash)))?;
                let new_name = module_config_name(hash);
                let config = path.with_file_name(format!("{}_config.toml", file_name));
                log::debug!(target: "migration", "renaming config {:?} to {}", config.file_name().unwrap(), new_name);
                std::fs::rename(&config, modules_dir.join(new_name))?;
            }
        };

        if let Err(e) = migrated {
            log::warn!("Module {:?} migration failed: {:?}", path, e);
        }
    }

    /// Adds a module to the filesystem, overwriting existing module.
    pub fn add_module(&self) -> Closure {
        let modules = self.modules_by_name.clone();
        let modules_dir = self.modules_dir.clone();
        closure(move |mut args| {
            let module: String = Args::next("module", &mut args)?;
            let module = base64::decode(&module).map_err(|err| {
                JValue::String(format!("error decoding module from base64: {:?}", err))
            })?;
            let hash = Hash::hash(&module);
            let config = Args::next("config", &mut args)?;
            let config = files::add_module(&modules_dir, &hash, &module, config)?;

            let hash_str = hash.to_hex().as_ref().to_owned();
            modules.lock().insert(config.name, hash);

            Ok(JValue::String(hash_str))
        })
    }

    /// Saves new blueprint to disk
    pub fn add_blueprint(&self) -> Closure {
        use Dependency::Hash;

        let blueprints_dir = self.blueprints_dir.clone();
        let modules = self.modules_by_name.clone();
        let blueprints = self.blueprints.clone();
        closure(move |mut args| {
            let blueprint: AddBlueprint = Args::next("blueprint_request", &mut args)?;

            if blueprint.dependencies.is_empty() {
                return Err(EmptyDependenciesList {
                    id: blueprint.name.clone(),
                }
                .into());
            }
            // resolve dependencies by name to hashes, if any
            let dependencies = blueprint.dependencies.into_iter();
            let dependencies: Vec<Dependency> = dependencies
                .map(|module| Ok(Hash(resolve_hash(&modules, module)?)))
                .collect::<Result<_>>()?;

            let hash = hash_dependencies(dependencies.clone())?.to_hex();

            let blueprint = Blueprint {
                id: hash.as_ref().to_string(),
                dependencies,
                name: blueprint.name,
            };
            files::add_blueprint(&blueprints_dir, &blueprint)?;

            blueprints
                .write()
                .insert(blueprint.id.clone(), blueprint.clone());

            Ok(JValue::String(blueprint.id))
        })
    }

    pub fn list_modules(&self) -> Closure {
        let modules_dir = self.modules_dir.clone();
        closure(move |_| {
            let modules = files::list_files(&modules_dir)
                .into_iter()
                .flatten()
                .filter_map(|path| {
                    let hash = extract_module_file_name(&path)?;
                    let result: eyre::Result<_> = try {
                        let hash = Hash::from_hex(hash).wrap_err(f!("invalid module name {path:?}"))?;
                        let config = modules_dir.join(module_config_name(&hash));
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
        })
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
        let modules_dir: PathBuf = self.modules_dir.clone();
        let cache: Arc<RwLock<HashMap<Hash, JValue>>> = self.module_interface_cache.clone();

        get_interface_by_hash(modules_dir, cache, hash)
    }

    pub fn get_interface(&self) -> Closure {
        let modules_dir: PathBuf = self.modules_dir.clone();
        let cache: Arc<RwLock<HashMap<Hash, JValue>>> = self.module_interface_cache.clone();

        closure(move |mut args| {
            let interface: eyre::Result<_> = try {
                let hash: String = Args::next("hash", &mut args)?;
                let hash = Hash::from_hex(&hash)?;

                get_interface_by_hash(modules_dir.clone(), cache.clone(), &hash)?
            };

            interface.map_err(|err| {
                json!(format!("{:?}", err)
                    // TODO: send patch to eyre so it can be done through their API
                    // Remove backtrace from the response
                    .split("Stack backtrace:")
                    .next()
                    .unwrap_or_default())
            })
        })
    }

    fn load_blueprints(blueprints_dir: &Path) -> HashMap<String, Blueprint> {
        let blueprints: Vec<Blueprint> = files::list_files(blueprints_dir)
            .into_iter()
            .flatten()
            .filter_map(|path| {
                // Check if file name matches blueprint schema
                let fname = path.file_name()?.to_str()?;
                if !file_names::is_blueprint(fname) {
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
    pub fn get_blueprints(&self) -> Closure {
        let blueprints = self.blueprints.clone();

        closure(move |_| {
            let blueprints: Vec<JValue> = blueprints
                .read()
                .values()
                .cloned()
                .map(|b| json!(b))
                .collect();
            Ok(JValue::Array(blueprints))
        })
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
    modules_dir: PathBuf,
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
            let path = modules_dir.join(module_file_name(hash));
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

fn hash_dependencies(deps: Vec<Dependency>) -> Result<Hash> {
    let mut hasher = blake3::Hasher::new();
    let mut deps: Vec<Hash> = deps
        .into_iter()
        .try_fold(vec![], |mut acc, dep| match dep {
            Dependency::Hash(h) => {
                acc.push(h);
                Ok(acc)
            }
            Dependency::Name(n) => Err(InvalidModuleReference {
                reference: n.to_string(),
            }),
        })?;

    deps.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

    for d in deps.iter() {
        hasher.update(d.as_bytes());
    }

    let hash = hasher.finalize();
    let bytes = hash.as_bytes();
    Ok(Hash::from(*bytes))
}

#[cfg(test)]
mod tests {
    use fluence_app_service::{TomlFaaSModuleConfig, TomlFaaSNamedModuleConfig};
    use host_closure::Args;
    use serde::{Deserialize, Serialize};
    use serde_json::Value as JValue;
    use tempdir::TempDir;
    use test_utils::{
        add_bp, add_module, load_module, response_to_return, Dependency, Hash, ModuleRepository,
        RetStruct,
    };

    fn get_interface(repo: &ModuleRepository, hash: String) -> RetStruct {
        let hash_v: JValue = serde_json::to_value(hash).unwrap();

        let args = Args {
            service_id: "".to_string(),
            function_name: "".to_string(),
            function_args: vec![hash_v],
            tetraplets: vec![],
        };

        let resp = repo.get_interface()(args);

        response_to_return(resp.unwrap())
    }

    fn get_bps(repo: &ModuleRepository) -> RetStruct {
        let args = Args {
            service_id: "".to_string(),
            function_name: "".to_string(),
            function_args: vec![],
            tetraplets: vec![],
        };

        let resp = repo.get_blueprints()(args);
        response_to_return(resp.unwrap())
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct BpsResult {
        dependencies: Vec<String>,
        id: String,
        name: String,
    }

    #[test]
    fn test_add_blueprint() {
        let module_dir = TempDir::new("test").unwrap();
        let bp_dir = TempDir::new("test").unwrap();
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path());

        let dep1 = Dependency::Hash(Hash::hash(&[1, 2, 3]));
        let dep2 = Dependency::Hash(Hash::hash(&[3, 2, 1]));

        let name1 = "bp1".to_string();
        let resp1 = add_bp(&repo, name1.clone(), vec![dep1.clone(), dep2.clone()]).unwrap();
        let bps1 = get_bps(&repo);
        let bps1: Vec<BpsResult> = serde_json::from_str(&bps1.result).unwrap();
        assert_eq!(bps1.len(), 1);
        let bp1 = bps1.get(0).unwrap();
        assert_eq!(bp1.name, name1);

        let name2 = "bp2".to_string();
        let resp2 = add_bp(&repo, "bp2".to_string(), vec![dep1, dep2]).unwrap();
        let bps2 = get_bps(&repo);
        let bps2: Vec<BpsResult> = serde_json::from_str(&bps2.result).unwrap();
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
        let repo = ModuleRepository::new(module_dir.path(), bp_dir.path());

        let module = load_module("../particle-node/tests/tetraplets/artifacts", "tetraplets");

        let config: TomlFaaSNamedModuleConfig = TomlFaaSNamedModuleConfig {
            name: "tetra".to_string(),
            file_name: None,
            config: TomlFaaSModuleConfig {
                mem_pages_count: None,
                logger_enabled: None,
                wasi: None,
                mounted_binaries: None,
                logging_mask: None,
            },
        };

        let hash = add_module(&repo, base64::encode(module), config).unwrap();

        let result = get_interface(&repo, hash);
        assert_eq!(0, result.ret_code);
    }

    #[test]
    fn test_hash_dependency() {
        use super::{hash_dependencies, Dependency};
        use crate::modules::Hash;

        let dep1 = Dependency::Hash(Hash::hash(&[1, 2, 3]));
        let dep2 = Dependency::Hash(Hash::hash(&[3, 2, 1]));

        let hash1 = hash_dependencies(vec![dep1.clone(), dep2.clone()]).unwrap();
        let hash2 = hash_dependencies(vec![dep2.clone(), dep1.clone()]).unwrap();
        assert_eq!(hash1.to_string(), hash2.to_string());
    }
}
