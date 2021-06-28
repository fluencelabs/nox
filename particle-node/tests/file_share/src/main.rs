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

use fluence::{get_call_parameters, marine, module_manifest};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::path::{Path, PathBuf};

module_manifest!();

pub fn main() {}

#[marine]
pub fn create_vault_file(contents: String) -> String {
    let name: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    let file = vault_dir().join(&name);
    std::fs::write(file, contents).expect("write");

    name
}

#[marine]
pub fn read_vault_file(filename: String) -> String {
    let file = vault_dir().join(filename);
    std::fs::read_to_string(file).expect("read")
}

fn vault_dir() -> PathBuf {
    let particle_id = get_call_parameters().particle_id;
    let vault = Path::new("/tmp").join(particle_id);

    vault
}
