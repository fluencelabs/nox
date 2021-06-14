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

use libp2p::PeerId;
use local_vm::{make_call_service_closure, make_particle, make_vm, read_args};
use particle_modules::{
    hash_dependencies, list_files, module_config_name_json, module_file_name, AddBlueprint,
    Dependency, Hash, NamedModuleConfig,
};

use aquamarine::{AquamarineApi, AVM};

use eyre::{eyre, ErrReport};
use eyre::{Result, WrapErr};
use futures::executor::block_on;
use maplit::hashmap;
use parking_lot::Mutex;
use serde_json::json;
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
struct ScheduledScript {
    pub name: String,
    pub data: String,
    pub interval_sec: u64,
}

#[derive(Debug)]
struct Module {
    // .wasm data
    pub data: Vec<u8>,
    // parsed json module config
    pub config: NamedModuleConfig,
}

#[derive(Debug)]
struct Builtin {
    // builtin alias
    pub name: String,
    // list of dependencies
    pub modules: Vec<Module>,
    pub blueprint: AddBlueprint,
    pub blueprint_id: String,
    pub on_start_script: Option<String>,
    pub on_start_data: Option<String>,
    pub scheduled_scripts: Vec<ScheduledScript>,
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

fn assert_ok(result: Vec<JValue>, err_msg: &str) -> eyre::Result<()> {
    match &result[..] {
        [JValue::String(s)] if s == "ok" => Ok(()),
        _ => Err(eyre!(err_msg.to_string())),
    }
}

fn load_modules(path: &PathBuf, dependencies: &Vec<Dependency>) -> Result<Vec<Module>> {
    let mut modules: Vec<Module> = vec![];
    for dep in dependencies.iter() {
        let config = path.join(Path::new(&module_config_name_json(dep)));
        let module = path.join(&module_file_name(dep));

        modules.push(Module {
            data: fs::read(module.clone()).wrap_err(eyre!("{:?} not found", module))?,
            config: serde_json::from_str(
                &fs::read_to_string(config.clone()).wrap_err(eyre!("{:?} not found", config))?,
            )?,
        });
    }

    Ok(modules)
}

fn load_blueprint(path: &PathBuf) -> Result<AddBlueprint> {
    Ok(serde_json::from_str(
        &fs::read_to_string(path.join("blueprint.json"))
            .wrap_err(eyre!("{:#?} not found", path))?,
    )?)
}

fn get_blueprint_id(modules: &Vec<Module>, name: String) -> Result<String> {
    let mut deps_hashes: Vec<Hash> = modules.iter().map(|m| Hash::hash(&m.data)).collect();
    let facade = deps_hashes.pop().ok_or(eyre!(
        "invalid blueprint {}: dependencies can't be empty",
        name
    ))?;

    Ok(hash_dependencies(facade, deps_hashes).to_string())
}

fn load_scheduled_scripts(path: &PathBuf) -> Result<Vec<ScheduledScript>> {
    let mut scripts = vec![];
    if let Some(files) = list_files(&path.join("scheduled")) {
        for path in files.into_iter() {
            let data = fs::read_to_string(path.clone())?;
            let name = path
                .file_stem()
                .ok_or(eyre!("invalid path"))?
                .to_str()
                .ok_or(eyre!("path to {} contain non-UTF-8 character"))?
                .to_string();

            let script_info: Vec<&str> = name.split("_").collect();
            let name = script_info
                .get(0)
                .ok_or(eyre!(
                    "invalid script naming, should be in {name}_{interval_in_sec}.air form"
                ))?
                .to_string();
            let interval_sec: u64 = script_info
                .get(1)
                .ok_or(eyre!(
                    "invalid script naming, should be in {name}_{interval_in_sec}.air form"
                ))?
                .parse()?;

            scripts.push(ScheduledScript {
                name,
                data,
                interval_sec,
            });
        }
    }

    Ok(scripts)
}

impl BuiltinsLoader {
    pub fn new(
        startup_peer_id: PeerId,
        node_peer_id: PeerId,
        node_api: AquamarineApi,
        base_dir: PathBuf,
    ) -> Self {
        let call_in = Arc::new(Mutex::new(hashmap! {}));
        let call_out = Arc::new(Mutex::new(vec![]));
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
        *self.call_service_in.lock() = data;
        self.call_service_in
            .lock()
            .insert("relay".to_string(), json!(self.node_peer_id.to_string()));

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
            .ok_or(eyre!("response doesn't contain particles".to_string()))?;

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
            "module_config".to_string() => json!(module.config),
        };

        let result = self
            .send_particle(script, data)
            .wrap_err("add_module call failed")?;

        assert_ok(result, "add_module call failed")
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

        let result = self
            .send_particle(script, hashmap! {"name".to_string() => json!(name)})
            .wrap_err("remove_service call failed")?;

        assert_ok(result, "remove_service call failed")
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

        let result = self
            .send_particle(script, data)
            .wrap_err("create_service call failed")?;

        assert_ok(result, "create_service call failed")
    }

    fn run_on_start(&mut self, builtin: &Builtin) -> eyre::Result<()> {
        if builtin.on_start_script.is_some() && builtin.on_start_data.is_some() {
            let data: HashMap<String, JValue> =
                serde_json::from_str(builtin.on_start_data.as_ref().unwrap())?;

            let res = self
                .send_particle(builtin.on_start_script.as_ref().unwrap().to_string(), data)
                .wrap_err("on_start call failed")?;
            return assert_ok(res, "on_start call failed");
        }

        Ok(())
    }

    fn run_scheduled_scripts(&mut self, builtin: &Builtin) -> eyre::Result<()> {
        for scheduled_script in builtin.scheduled_scripts.iter() {
            let script = r#"
            (xor
                (seq
                    (call relay ("script" "add") [script interval_sec])
                    (call %init_peer_id% ("op" "return") ["ok"])
                )
                (call %init_peer_id% ("op" "return") [%last_error%.$.instruction])
            )
            "#
            .to_string();

            let data = hashmap! {
                "script".to_string() => json!(scheduled_script.data),
                "interval_sec".to_string() => json!(scheduled_script.interval_sec),
            };

            let res = self.send_particle(script, data).wrap_err(format!(
                "scheduled script {} run failed",
                scheduled_script.name
            ))?;

            assert_ok(
                res,
                &format!("scheduled script {} run failed", scheduled_script.name),
            )?;
        }

        Ok(())
    }

    pub fn deploy_builtin_services(&mut self) -> Result<()> {
        let available_builtins = self.list_builtins()?;
        let local_services = self.get_service_blueprints()?;

        for builtin in available_builtins.iter() {
            let result: Result<()> = try {
                match local_services.get(&builtin.name) {
                    Some(id) if *id == builtin.blueprint_id => {
                        self.run_on_start(builtin)?;
                        self.run_scheduled_scripts(&builtin)?;
                        continue;
                    }
                    Some(_) => self.remove_service(builtin.name.clone())?,
                    _ => {}
                }

                for module in builtin.modules.iter() {
                    self.add_module(module)?;
                }

                self.create_service(&builtin)?;
                self.run_on_start(builtin)?;
                self.run_scheduled_scripts(&builtin)?;
            };

            if let Err(err) = result {
                log::error!("builtin {} init is failed: {}", builtin.name, err);
            }
        }

        Ok(())
    }

    fn list_builtins(&self) -> Result<Vec<Builtin>> {
        let (successful, failed): (Vec<Builtin>, Vec<ErrReport>) =
            list_files(self.builtins_base_dir.as_path())
                .ok_or(eyre!("{:#?} folder not found", self.builtins_base_dir))?
                .fold(
                    (vec![], vec![]),
                    |(mut successful, mut failed): (Vec<Builtin>, Vec<ErrReport>), path| {
                        let result = try {
                            let name = path
                                .file_name()
                                .ok_or(eyre!(""))?
                                .to_str()
                                .ok_or(eyre!(""))?
                                .to_string();

                            let blueprint = load_blueprint(&path)?;
                            let modules = load_modules(&path, &blueprint.dependencies)?;
                            let blueprint_id = get_blueprint_id(&modules, name.clone())?;
                            let scheduled_scripts = load_scheduled_scripts(&path)?;

                            Builtin {
                                name,
                                modules,
                                blueprint,
                                blueprint_id,
                                on_start_script: fs::read_to_string(path.join("on_start.air")).ok(),
                                on_start_data: fs::read_to_string(path.join("on_start.json")).ok(),
                                scheduled_scripts,
                            }
                        };

                        match result {
                            Ok(builtin) => successful.push(builtin),
                            Err(err) => failed.push(err),
                        }
                        (successful, failed)
                    },
                );

        failed
            .into_iter()
            .map(|err| log::error!("builtin load failed: {:#}", err))
            .for_each(drop);

        Ok(successful)
    }

    fn get_service_blueprints(&mut self) -> Result<HashMap<String, String>> {
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

        let result = self
            .send_particle(script, hashmap! {})
            .wrap_err("srv list call failed")?;
        let result = match result.get(0) {
            Some(JValue::Array(result)) => result,
            _ => Err(eyre!("list_services call failed"))?,
        };

        let mut blueprint_ids = hashmap! {};

        for p in result.into_iter() {
            let blueprint_id = match p.get("blueprint_id") {
                Some(JValue::String(id)) => id,
                _ => Err(eyre!("list_services call failed"))?,
            };

            let aliases = match p.get("aliases") {
                Some(JValue::Array(aliases)) => aliases,
                _ => Err(eyre!("list_services call failed"))?,
            };

            for alias in aliases.into_iter() {
                let alias = alias
                    .as_str()
                    .ok_or(eyre!("list_services call failed"))?
                    .to_string();
                blueprint_ids.insert(alias, blueprint_id.clone());
            }
        }

        Ok(blueprint_ids)
    }
}
