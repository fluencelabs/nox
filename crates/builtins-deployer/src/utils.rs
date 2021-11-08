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

use std::path::Path;
use std::{env, fs};

use eyre::{eyre, Result, WrapErr};
use regex::Regex;
use serde_json::Value as JValue;

use fs_utils::file_stem;
use particle_modules::{list_files, AddBlueprint};
use service_modules::{
    hash_dependencies, module_config_name_json, module_file_name, Dependency, Hash,
};

use crate::builtin::{Module, ScheduledScript};
use crate::ALLOWED_ENV_PREFIX;

pub fn assert_ok(result: Vec<JValue>, err_msg: &str) -> eyre::Result<()> {
    match &result[..] {
        [JValue::String(s)] if s == "ok" => Ok(()),
        [JValue::Bool(true)] => Ok(()),
        _ => Err(eyre!("{}: {:?}", err_msg.to_string(), result)),
    }
}

pub fn load_modules(path: &Path, dependencies: &[Dependency]) -> Result<Vec<Module>> {
    let mut modules: Vec<Module> = vec![];
    for dep in dependencies.iter() {
        let config = path.join(Path::new(&module_config_name_json(dep)));
        let module = path.join(&module_file_name(dep));

        modules.push(Module {
            data: fs::read(module.clone()).wrap_err(eyre!("module {:?} not found", module))?,
            config: serde_json::from_str(
                &fs::read_to_string(config.clone())
                    .wrap_err(eyre!("config {:?} not found", config))?,
            )?,
        });
    }

    Ok(modules)
}

pub fn load_blueprint(path: &Path) -> Result<AddBlueprint> {
    Ok(serde_json::from_str(
        &fs::read_to_string(path.join("blueprint.json"))
            .wrap_err(eyre!("blueprint {:?} not found", path))?,
    )?)
}

pub fn get_blueprint_id(modules: &[Module], name: String) -> Result<String> {
    let mut deps_hashes: Vec<Hash> = modules.iter().map(|m| Hash::hash(&m.data)).collect();
    let facade = deps_hashes
        .pop()
        .ok_or_else(|| eyre!("invalid blueprint {}: dependencies can't be empty", name))?;

    Ok(hash_dependencies(facade, deps_hashes).to_string())
}

pub fn load_scheduled_scripts(path: &Path) -> Result<Vec<ScheduledScript>> {
    let mut scripts = vec![];
    if let Some(files) = list_files(&path.join("scheduled")) {
        for path in files.into_iter() {
            let data = fs::read_to_string(path.to_path_buf())?;
            let name = file_stem(&path)?;

            let mut script_info = name.split('_');
            let name = script_info
                .next()
                .ok_or_else(|| {
                    eyre!(
                        "invalid script name {}, should be in %name%_%interval_in_sec%.air form",
                        name
                    )
                })?
                .to_string();
            let interval_sec: u64 = script_info
                .next()
                .ok_or_else(|| {
                    eyre!(
                        "invalid script name {}, should be in %name%_%interval_in_sec%.air form",
                        name
                    )
                })?
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

pub fn resolve_env_variables(data: &str, service_name: &str) -> Result<String> {
    let mut result = data.to_string();
    let env_prefix = format!(
        "{}_{}",
        ALLOWED_ENV_PREFIX,
        service_name.to_uppercase().replace('-', "_")
    );

    let re = Regex::new(&f!(r"(\{env_prefix}_\w+)"))?;
    for elem in re.captures_iter(data) {
        result = result.replace(
            &elem[0],
            &env::var(&elem[0][1..]).map_err(|e| eyre!("{}: {}", e.to_string(), &elem[0][1..]))?,
        );
    }

    Ok(result)
}
