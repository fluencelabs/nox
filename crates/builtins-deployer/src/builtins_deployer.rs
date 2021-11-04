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

use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, fs};

use eyre::{eyre, ErrReport, Result, WrapErr};
use futures::channel::oneshot::channel;
use futures::executor::block_on;
use futures::FutureExt;
use maplit::hashmap;
use serde_json::{json, Value as JValue};

use aquamarine::AquamarineApi;
use fluence_libp2p::PeerId;
use fs_utils::{file_name, to_abs_path};
use local_vm::{client_functions, wrap_script};
use now_millis::now_ms;
use particle_modules::list_files;
use particle_protocol::Particle;
use uuid_utils::uuid;

use crate::builtin::{Builtin, Module};
use crate::utils::{
    assert_ok, get_blueprint_id, load_blueprint, load_modules, load_scheduled_scripts,
    resolve_env_variables,
};

pub struct BuiltinsDeployer {
    startup_peer_id: PeerId,
    node_peer_id: PeerId,
    aquamarine: AquamarineApi,
    builtins_base_dir: PathBuf,
    particle_ttl: Duration,
    // if set to true, remove existing builtins before deploying
    force_redeploy: bool,
    // the number of ping attempts to check the readiness of the vm pool
    retry_attempts_count: u16,
}

impl BuiltinsDeployer {
    pub fn new(
        startup_peer_id: PeerId,
        node_peer_id: PeerId,
        aquamarine: AquamarineApi,
        base_dir: PathBuf,
        particle_ttl: Duration,
        force_redeploy: bool,
        retry_attempts_count: u16,
    ) -> Self {
        Self {
            startup_peer_id,
            node_peer_id,
            aquamarine,
            builtins_base_dir: base_dir,
            particle_ttl,
            force_redeploy,
            retry_attempts_count,
        }
    }

    fn send_particle(
        &mut self,
        script: String,
        mut data: HashMap<String, JValue>,
    ) -> eyre::Result<Vec<JValue>> {
        data.insert("node".to_string(), json!(self.node_peer_id.to_string()));
        data.insert("relay".to_string(), json!(self.node_peer_id.to_string()));

        // TODO: set to true if AIR script is generated from Aqua
        let script = wrap_script(script, &data, None, false, Some(self.node_peer_id));
        let (outlet, inlet) = channel();

        let mut outlet = Some(outlet);
        let closure = move |args, _| {
            let result = client_functions(&data, args);

            if let Some(returned) = result.returned {
                if let Some(outlet) = outlet.take() {
                    outlet.send(returned).expect("send response back")
                } else {
                    log::info!("WTF!")
                }
            }

            let outcome = result.outcome;
            async { outcome }.boxed()
        };
        let aquamarine = self.aquamarine.clone();

        let particle = Particle {
            id: uuid(),
            init_peer_id: self.startup_peer_id,
            timestamp: now_ms() as u64,
            ttl: self.particle_ttl.as_millis() as u32,
            script,
            signature: vec![],
            data: vec![],
        };

        let future = async move {
            try {
                aquamarine
                    .execute(particle, Some(Box::new(closure)))
                    .await?;

                let result = inlet.await;
                result
                    .map_err(|err| eyre!("error reading from inlet: {:?}", err))?
                    .map_err(|args| eyre!("AIR caught an error on args: {:?}", args))?
            }
        };

        block_on(future)
    }

    fn add_module(&mut self, module: &Module) -> eyre::Result<()> {
        let script = r#"
        (xor
            (seq
                (call node ("dist" "add_module") [module_bytes module_config])
                (call node ("op" "return") [true])
            )
            (call node ("op" "return") [%last_error%.$.instruction])
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
                (call node ("srv" "remove") [name])
                (call node ("op" "return") [true])
            )
            (call node ("op" "return") [%last_error%.$.instruction])
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
                (call node ("dist" "add_blueprint") [blueprint] blueprint_id)
                (seq
                    (call node ("srv" "create") [blueprint_id] service_id)
                    (seq
                        (call node ("srv" "add_alias") [alias service_id] result)
                        (call node ("op" "return") [true])
                    )
                )
            )
            (call node ("op" "return") [%last_error%.$.instruction])
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

        assert_ok(result, "create_service call failed")?;

        log::info!("service was created!");

        Ok(())
    }

    // TODO: right now, if AIR in on_start.air is invalid, everything just hangs
    fn run_on_start(&mut self, builtin: &Builtin) -> eyre::Result<()> {
        if builtin.on_start_script.is_some() && builtin.on_start_data.is_some() {
            let data: HashMap<String, JValue> = serde_json::from_str(&resolve_env_variables(
                builtin.on_start_data.as_ref().unwrap(),
                &builtin.name,
            )?)?;

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
                    (call node ("script" "add") [script interval_sec])
                    (call node ("op" "return") [true])
                )
                (call node ("op" "return") [%last_error%.$.instruction])
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

    fn wait_for_vm_pool(&mut self) -> Result<()> {
        let mut attempt = 0u16;
        loop {
            attempt += 1;

            let result: eyre::Result<()> = try {
                let script = r#"
                    (seq
                        (null)
                        (call node ("op" "return") [true])
                    )
                    "#
                .to_string();

                let res = self
                    .send_particle(script, hashmap! {})
                    .map_err(|e| eyre::eyre!("ping send_particle #{} failed: {}", attempt, e))?;

                assert_ok(res, &format!("ping call #{} failed", attempt))?
            };

            if let Err(err) = result {
                log::warn!("Attempt to ping vm pool failed: {}", err);

                if attempt > self.retry_attempts_count {
                    return Err(eyre::eyre!(
                        "Attempts limit exceeded. Can't connect to vm pool: {}",
                        err
                    ));
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    pub fn deploy_builtin_services(&mut self) -> Result<()> {
        let from_disk = self.list_builtins()?;
        if from_disk.is_empty() {
            log::info!("No builtin services found at {:?}", self.builtins_base_dir);
            return Ok(());
        }

        self.wait_for_vm_pool()?;

        let mut local_services = self.get_service_blueprints()?;

        let mut to_create = vec![];
        let mut to_start = vec![];

        // if force_redeploy is set, then first remove all builtins
        if self.force_redeploy {
            for builtin in from_disk.iter() {
                if local_services.contains_key(&builtin.name) {
                    self.remove_service(builtin.name.clone())?;
                    local_services.remove(&builtin.name);
                }
            }
        }

        for builtin in from_disk.iter() {
            // check if builtin is already deployed
            match local_services.get(&builtin.name) {
                // already deployed
                // if blueprint_id has changed, then redeploy builtin
                Some(bp_id) if *bp_id != builtin.blueprint_id => {
                    self.remove_service(builtin.name.clone())?;
                    to_create.push(builtin)
                }
                // already deployed with expected blueprint_id
                Some(_) => {
                    to_start.push(builtin);
                }
                // isn't deployed yet
                None => to_create.push(builtin),
            }
        }

        for builtin in to_create {
            let result: Result<()> = try {
                self.upload_modules(builtin)?;
                self.create_service(builtin)?;
                to_start.push(builtin);
            };

            if let Err(err) = result {
                log::error!("builtin {} init is failed: {}", builtin.name, err);
                return Err(err);
            }
        }

        for builtin in to_start.into_iter() {
            self.run_on_start(builtin)?;
            self.run_scheduled_scripts(builtin)?;
        }

        Ok(())
    }

    fn upload_modules(&mut self, builtin: &Builtin) -> Result<()> {
        for module in builtin.modules.iter() {
            self.add_module(module)
                .wrap_err(format!("builtin {} module upload failed", builtin.name))?;
        }

        Ok(())
    }

    fn list_builtins(&self) -> Result<Vec<Builtin>> {
        let builtins_dir = to_abs_path(self.builtins_base_dir.clone());
        let builtins = list_files(&builtins_dir)
            .ok_or_else(|| eyre!("{:#?} directory not found", builtins_dir))?
            .filter(|p| p.is_dir());

        let (successful, failed): (Vec<Builtin>, Vec<ErrReport>) = builtins.fold(
            (vec![], vec![]),
            |(mut successful, mut failed): (Vec<Builtin>, Vec<ErrReport>), path| {
                let result = try {
                    let name = file_name(&path)?;
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
            .iter()
            .map(|err| {
                log::error!("builtin load failed: {:#}", err);
            })
            .for_each(drop);

        return if !failed.is_empty() {
            Err(eyre!(
                "failed to load builtins from disk {:?}",
                builtins_dir
            ))
        } else {
            Ok(successful)
        };
    }

    fn get_service_blueprints(&mut self) -> Result<HashMap<String, String>> {
        let script = r#"
        (xor
            (seq
                (call node ("srv" "list") [] list)
                (call node ("op" "return") [list])
            )
            (call node ("op" "return") [%last_error%.$.instruction])
        )
        "#
        .to_string();

        let result = self
            .send_particle(script, hashmap! {})
            .wrap_err("srv list call failed")?;
        let result = match result.get(0) {
            Some(JValue::Array(result)) => result,
            _ => return Err(eyre!("list_services call failed")),
        };

        let mut blueprint_ids = hashmap! {};

        for p in result.iter() {
            let blueprint_id = match p.get("blueprint_id") {
                Some(JValue::String(id)) => id,
                _ => return Err(eyre!("list_services call failed")),
            };

            let aliases = match p.get("aliases") {
                Some(JValue::Array(aliases)) => aliases,
                _ => return Err(eyre!("list_services call failed")),
            };

            for alias in aliases.iter() {
                let alias = alias
                    .as_str()
                    .ok_or_else(|| eyre!("list_services call failed"))?
                    .to_string();
                blueprint_ids.insert(alias, blueprint_id.clone());
            }
        }

        Ok(blueprint_ids)
    }
}
