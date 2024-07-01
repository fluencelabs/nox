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
