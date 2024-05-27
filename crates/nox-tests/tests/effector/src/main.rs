/*
 * Copyright 2024 Fluence DAO
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

use marine_rs_sdk::marine;
use marine_rs_sdk::module_manifest;
use marine_rs_sdk::MountedBinaryResult;

module_manifest!();

fn main() { }

#[marine]
pub struct Result {
    success: bool,
    result: Vec<String>,
    error: Vec<String>,
}

#[marine]
fn list_directory(path: String) -> Result {
    let result = ls(vec![inject_vault_host_path(path)]);
    if let Some(result) = result.into_std() {
        match result {
            Ok(out) => Result { error: vec![], result: vec![out], success: true },
            Err(err) => Result { error: vec![err.to_string()], result: vec![], success: false},
        }
    } else {
        Result { error: vec!["MountedBinaryResult::into_std return None".to_string()], result: vec![], success: true }
    }
}

#[marine]
#[host_import]
extern "C" {
    fn ls(cmd: Vec<String>) -> MountedBinaryResult;
}

fn inject_vault_host_path(path: String) -> String {
    let vault = "/tmp/vault";
    if let Some(stripped) = path.strip_prefix(&vault) {
        let host_vault_path = std::env::var(vault).expect("vault must be mapped to /tmp/vault");
        format!("/{}/{}", host_vault_path, stripped)
    } else {
        path
    }
}
