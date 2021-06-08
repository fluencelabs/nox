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
use eyre::Result;
use futures::executor::block_on;
use libp2p::PeerId;
use local_vm::{make_call_service_closure, make_particle, make_vm, read_args};
use maplit::hashmap;
use parking_lot::Mutex;
use particle_modules::{list_files, AddBlueprint};
use serde_json::json;
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Module {
    pub data: Vec<u8>,
    pub config: String,
}

struct Builtin {
    pub name: String,
    pub modules: Vec<Module>,
    pub blueprint: AddBlueprint,
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

    // pub fn get_blueprint_id(builtin: &Builtin) {
    //     let mut deps = builtin.blueprint.dependencies.clone();
    //     let facade = deps.pop().unwrap();
    //     hash_dependencies(facade, deps)
    // }

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
            "relay" => json!(self.node_peer_id.to_string()),
            "module_bytes" => json!(base64::encode(&module.data)),
            "module_config" => serde_json::from_str(&module.config)?,
        };

        *self.call_service_in.lock() = data
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        let particle = make_particle(
            self.startup_peer_id.clone(),
            self.call_service_in.clone(),
            script,
            None,
            &mut self.local_vm,
        );

        let result = block_on(self.node_api.clone().handle(particle))?;

        for pt in result.particles {
            let res = read_args(
                pt.particle,
                self.startup_peer_id.clone(),
                &mut self.local_vm,
                self.call_service_out.clone(),
            );

            for v in res.into_iter() {
                log::info!("{}", v.to_string());
            }
        }

        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        let available_builtins = self.list_builtins()?;
        let local_services = self.list_services()?;
        // for _builtin in available_builtins.into_iter() {}
        println!("{:?}", local_services);
        println!(
            "{:?}",
            available_builtins
                .iter()
                .map(|e| e.blueprint.clone())
                .collect::<Vec<AddBlueprint>>()
        );

        for builtin in available_builtins.iter() {
            // let old_blueprint = local_services.get(&builtin.name);
            //
            // if old_blueprint.is_some() && old_blueprint.unwrap() == builtin.blueprint.

            for module in builtin.modules.iter() {
                self.add_module(module)?;
            }
        }

        Ok(())
    }

    fn list_builtins(&self) -> Result<Vec<Builtin>> {
        list_files(self.builtins_base_dir.as_path())
            .into_iter()
            .flatten()
            .try_fold(vec![], |mut acc: Vec<Builtin>, path: PathBuf| {
                let builtin: Result<Builtin> = try {
                    let path = path.as_path();
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();
                    let blueprint =
                        serde_json::from_str(&fs::read_to_string(path.join("blueprint.json"))?)?;

                    let modules_names = list_files(path)
                        .unwrap()
                        .into_iter()
                        .filter(|p| p.extension().unwrap().eq("wasm"))
                        .map(|m| m.file_stem().unwrap().to_str().unwrap().to_string())
                        .collect::<Vec<String>>();

                    let mut modules: Vec<Module> = vec![];
                    for module in modules_names.into_iter() {
                        let config_name = module.clone() + "_cfg.json";
                        let config = self
                            .builtins_base_dir
                            .join(name.clone())
                            .join(Path::new(&config_name));
                        let module = self
                            .builtins_base_dir
                            .join(name.clone())
                            .join(module.clone() + ".wasm");

                        modules.push(Module {
                            data: fs::read(module)?,
                            config: fs::read_to_string(config)?,
                        })
                    }

                    Builtin {
                        name,
                        modules,
                        blueprint,
                    }
                };

                match builtin {
                    Ok(builtin) => {
                        acc.push(builtin);
                        Ok(acc)
                    }
                    Err(err) => Err(err),
                }
            })
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

        let data = hashmap! {
            "relay" => json!(self.node_peer_id.to_string()),
        };

        *self.call_service_in.lock() = data
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect();

        let particle = make_particle(
            self.startup_peer_id.clone(),
            self.call_service_in.clone(),
            script,
            None,
            &mut self.local_vm,
        );

        let result = block_on(self.node_api.clone().handle(particle))?;

        let mut blueprint_ids = hashmap! {};
        for pt in result.particles {
            let res = read_args(
                pt.particle,
                self.startup_peer_id.clone(),
                &mut self.local_vm,
                self.call_service_out.clone(),
            );

            for v in res.into_iter() {
                for p in v.as_array().unwrap().iter() {
                    let blueprint_id = p.get("blueprint_id").unwrap().as_str().unwrap().to_string();
                    for alias in p.get("aliases").unwrap().as_array().unwrap().iter() {
                        blueprint_ids
                            .insert(alias.as_str().unwrap().to_string(), blueprint_id.clone());
                    }
                }
            }
        }

        Ok(blueprint_ids)
    }
}
