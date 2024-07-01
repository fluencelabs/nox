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

use fs_utils::to_abs_path;

use eyre::{Result, WrapErr};
use serde_json::{json, Value as JValue};
use std::path::PathBuf;

pub fn load_module(path: &str, module_name: impl Into<String>) -> Result<Vec<u8>> {
    let module_name = format!("{}.wasm", module_name.into());
    let module = to_abs_path(PathBuf::from(path).join(module_name));
    std::fs::read(&module).wrap_err(format!("failed to load module {module:?}"))
}

pub fn module_config(import_name: &str) -> JValue {
    json!(
    {
        "name": import_name,
        "mem_pages_count": 100,
        "logger_enabled": true,
        "preopened_files": vec!["/tmp"],
        "wasi": {
            "envs": json!({}),
            "mapped_dirs": json!({}),
        }
    })
}
