/*
 * Copyright 2021 Fluence Labs Limited
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

use aquamarine::{AquamarineApi, AVM};
use boolinator::Boolinator;
use eyre::Result;
use futures::executor::block_on;
use libp2p::PeerId;
use local_vm::{make_call_service_closure, make_particle, make_vm, read_args};
use maplit::hashmap;
use parking_lot::Mutex;
use particle_modules::{hash_dependencies, list_files, AddBlueprint, Dependency, Hash};
use serde_json::json;
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, iter};

#[derive(Debug)]
struct Module {
    pub data: Vec<u8>,
    pub config: String,
}

#[derive(Debug)]
struct Builtin {
    pub name: String,
    pub modules: Vec<Module>,
    pub blueprint: AddBlueprint,
    pub blueprint_id: String,
    pub on_start_script: Option<String>,
    pub on_start_data: Option<String>,
}

pub struct BuiltinsLoader {
    startup_peer_id: PeerId,
    node_peer_id: PeerId,
    node_api: AquamarineApi,
    local_vm: AVM,
    call_service_in: Arc<Mutex<HashMap<String, JValue>>>,
    call_service_out: Arc<Mutex<Vec<JValue>>>,
    builtins_base_dir: PathBuf,
}

fn check_result(result: Vec<JValue>, err_msg: &str) -> eyre::Result<()> {
    let result = result
        .get(0)
        .ok_or(eyre::eyre!(err_msg.to_string()))?
        .as_str()
        .ok_or(eyre::eyre!(err_msg.to_string()))?
        .to_string();

    result.eq("ok").ok_or(eyre::eyre!(err_msg.to_string()))
}

impl BuiltinsLoader {
    pub fn new(
        startup_peer_id: PeerId,
        node_peer_id: PeerId,
        node_api: AquamarineApi,
        base_dir: PathBuf,
    ) -> Self {
        let call_in: Arc<Mutex<HashMap<String, JValue>>> = <_>::default();
        let call_out: Arc<Mutex<Vec<JValue>>> = <_>::default();
        Self {
            startup_peer_id,
            node_peer_id,
            node_api,
            local_vm: make_vm(
                startup_peer_id,
                make_call_service_closure(call_in.clone(), call_out.clone()),
            ),
            call_service_in: call_in,
            call_service_out: call_out,
            builtins_base_dir: base_dir,
        }
    }

    fn send_particle(
        &mut self,
        script: String,
        data: HashMap<String, JValue>,
    ) -> eyre::Result<Vec<JValue>> {
        *self.call_service_in.lock() = data
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .chain(iter::once((
                "relay".to_string(),
                json!(self.node_peer_id.to_string()),
            )))
            .collect();

        let particle = make_particle(
            self.startup_peer_id.clone(),
            self.call_service_in.clone(),
            script,
            None,
            &mut self.local_vm,
        );

        let result = block_on(self.node_api.clone().handle(particle))?;

        let particle = result
            .particles
            .get(0)
            .ok_or(eyre::eyre!("response doesn't contain particles".to_string()))?;

        Ok(read_args(
            particle.particle.clone(),
            self.startup_peer_id.clone(),
            &mut self.local_vm,
            self.call_service_out.clone(),
        ))
    }

    fn add_module(&mut self, module: &Module) -> eyre::Result<()> {
        let script = r#"
        (xor
            (seq
                (call relay ("dist" "add_module") [module_bytes module_config])
                (call %init_peer_id% ("op" "return") ["ok"])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
        "#
        .to_string();

        let data = hashmap! {
            "module_bytes".to_string() => json!(base64::encode(&module.data)),
            "module_config".to_string() => serde_json::from_str(&module.config)?,
        };

        let result = self.send_particle(script, data)?;

        check_result(result, "add_module call failed")
    }

    fn remove_service(&mut self, name: String) -> eyre::Result<()> {
        let script = r#"
        (xor
            (seq
                (call relay ("srv" "remove") [name])
                (call %init_peer_id% ("op" "return") ["ok"])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
        "#
        .to_string();

        let result = self.send_particle(script, hashmap! {"name".to_string() => json!(name)})?;

        check_result(result, "remove_service call failed")
    }

    fn create_service(&mut self, builtin: &Builtin) -> eyre::Result<()> {
        let script = r#"
        (xor
            (seq
                (call relay ("dist" "add_blueprint") [blueprint] blueprint_id)
                (seq
                    (call relay ("srv" "create") [blueprint_id] service_id)
                    (seq
                        (call relay ("srv" "add_alias") [alias service_id] result)
                        (call %init_peer_id% ("op" "return") ["ok"])
                    )
                )
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
        "#
        .to_string();

        let data = hashmap! {
            "blueprint".to_string() => json!(builtin.blueprint),
            "alias".to_string() => json!(builtin.name),
        };

        let result = self.send_particle(script, data)?;

        check_result(result, "create_service call failed")
    }

    fn run_on_start(&mut self, builtin: &Builtin) -> eyre::Result<()> {
        if builtin.on_start_script.is_some() && builtin.on_start_data.is_some() {
            let data: HashMap<String, JValue> =
                serde_json::from_str(builtin.on_start_data.as_ref().unwrap())?;

            let res =
                self.send_particle(builtin.on_start_script.as_ref().unwrap().to_string(), data)?;
            return check_result(res, "on_start call failed");
        }

        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        let available_builtins = self.list_builtins()?;
        let local_services = self.list_services()?;

        for builtin in available_builtins.iter() {
            let result: Result<()> = try {
                let old_blueprint = local_services.get(&builtin.name);
                if let Some(id) = old_blueprint {
                    if *id == builtin.blueprint_id {
                        self.run_on_start(builtin)?;
                        continue;
                    } else {
                        self.remove_service(builtin.name.clone())?;
                    }
                }

                for module in builtin.modules.iter() {
                    self.add_module(module)?;
                }

                self.create_service(&builtin)?;
                self.run_on_start(builtin)?;
            };

            if let Err(err) = result {
                log::error!("builtin {} init is failed: {}", builtin.name, err);
            }
        }

        Ok(())
    }

    fn list_builtins(&self) -> Result<Vec<Builtin>> {
        let (success, failed): (Vec<Result<Builtin>>, Vec<Result<Builtin>>) =
            list_files(self.builtins_base_dir.as_path())
                .ok_or(eyre::eyre!("builtins folder not found"))?
                .map(|path| {
                    let path = path.as_path();
                    let name = path
                        .file_name()
                        .ok_or(eyre::eyre!(""))?
                        .to_str()
                        .ok_or(eyre::eyre!(""))?
                        .to_string();
                    let blueprint: AddBlueprint =
                        serde_json::from_str(&fs::read_to_string(path.join("blueprint.json"))?)?;

                    let mut modules: Vec<Module> = vec![];
                    for module_name in blueprint.dependencies.iter() {
                        match module_name {
                            Dependency::Name(module_name) => {
                                let config_name = module_name.clone() + "_cfg.json";
                                let config = self
                                    .builtins_base_dir
                                    .join(name.clone())
                                    .join(Path::new(&config_name));
                                let module = self
                                    .builtins_base_dir
                                    .join(name.clone())
                                    .join(module_name.clone() + ".wasm");

                                modules.push(Module {
                                    data: fs::read(module)?,
                                    config: fs::read_to_string(config)?,
                                });
                            }
                            _ => {
                                return Err(eyre::eyre!(
                            "incorrect blueprint for {}: dependencies should contain only names",
                            name
                        ));
                            }
                        }
                    }

                    let mut deps_hashes: Vec<Hash> =
                        modules.iter().map(|m| Hash::hash(&m.data)).collect();
                    let facade = deps_hashes.pop().ok_or(eyre::eyre!(""))?;

                    Ok(Builtin {
                        name,
                        modules,
                        blueprint,
                        blueprint_id: hash_dependencies(facade, deps_hashes)?.to_string(),
                        on_start_script: fs::read_to_string(path.join("on_start.air")).ok(),
                        on_start_data: fs::read_to_string(path.join("on_start.json")).ok(),
                    })
                })
                .partition(Result::is_ok);

        failed
            .into_iter()
            .map(Result::unwrap_err)
            .map(|err| log::error!("builtin load failed: {}", err.to_string()))
            .for_each(drop);

        Ok(success.into_iter().map(Result::unwrap).collect())
    }

    fn list_services(&mut self) -> Result<HashMap<String, String>> {
        let script = r#"
        (xor
            (seq
                (call relay ("srv" "list") [] list)
                (call %init_peer_id% ("op" "return") [list])
            )
            (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
        )
        "#
        .to_string();

        let result = self.send_particle(script, hashmap! {})?;
        let result = result
            .get(0)
            .ok_or(eyre::eyre!("list_services call failed: response is empty"))?
            .as_array()
            .ok_or(eyre::eyre!("list_services call failed"))?;
        let mut blueprint_ids = hashmap! {};

        for p in result.into_iter() {
            let blueprint_id = p
                .get("blueprint_id")
                .ok_or(eyre::eyre!("list_services call failed"))?
                .as_str()
                .ok_or(eyre::eyre!("list_services call failed"))?
                .to_string();
            let aliases = p
                .get("aliases")
                .ok_or(eyre::eyre!("list_services call failed"))?
                .as_array()
                .ok_or(eyre::eyre!("list_services call failed"))?;

            for alias in aliases.into_iter() {
                let alias = alias
                    .as_str()
                    .ok_or(eyre::eyre!("list_services call failed"))?
                    .to_string();
                blueprint_ids.insert(alias, blueprint_id.clone());
            }
        }

        Ok(blueprint_ids)
    }
}
