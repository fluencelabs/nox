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

use crate::modules::dependency::Dependency;
use crate::modules::file_names::module_file_name;

use fs_utils::to_abs_path;

use eyre::{Result, WrapErr};
use serde_json::{json, Value as JValue};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn load_module(path: &str, module_name: impl Into<String>) -> Result<Vec<u8>> {
    let module_name = module_file_name(&Dependency::Name(module_name.into()));
    let module = to_abs_path(PathBuf::from(path).join(module_name));
    std::fs::read(&module).wrap_err(format!("failed to load module {:?}", module))
}

pub fn module_config(import_name: &str) -> JValue {
    serde_json::json!(
        {
            "name": import_name,
            "mem_pages_count": 100,
            "logger_enabled": true,
            "wasi": {
                "envs": json!({}),
                "preopened_files": vec!["/tmp"],
                "mapped_dirs": json!({}),
            }
        }
    )
}

pub fn module_config_map(name: &str) -> HashMap<&str, JValue> {
    maplit::hashmap! {
        "module_name" => json!(name),
        "mem_pages_count" => json!(100),
        "logger_enabled" => json!(true),
        "preopened_files" => json!(["/tmp"]),
        "envs" => json!([]),
        "mapped_dirs" => json!([]),
        "mounted_binaries" => json!([]),
        "logging_mask" => JValue::Null,
    }
}
