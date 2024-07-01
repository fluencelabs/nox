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

use base64::{engine::general_purpose::STANDARD as base64, Engine};
use marine_rs_sdk::{get_call_parameters, marine, module_manifest};
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::path::{Path, PathBuf};

module_manifest!();

pub fn main() {}

#[marine]
pub fn create_vault_file(contents: String) -> String {
    store_file_name(&contents)
}

#[marine]
pub fn create_vault_file_path(contents: String) -> String {
    store_file_path(&contents)
}

#[marine]
pub fn read_vault_file(filename: String) -> String {
    let file = vault_dir().join(filename);
    std::fs::read_to_string(file).expect("read")
}

#[marine]
pub fn read_base64_vault_file(filename: String) -> String {
    let file = vault_dir().join(filename);
    let bytes = match std::fs::read(file) {
        Ok(bs) => bs,
        Err(err) => return err.to_string(),
    };
    base64.encode(bytes)
}

#[marine]
pub fn create_base64_vault_file(data: String) -> String {
    let bytes = base64.decode(data).expect("correct base64");
    store_file_name(bytes)
}

fn store_file_name(contents: impl AsRef<[u8]>) -> String {
    store_file(contents).0
}

fn store_file_path(contents: impl AsRef<[u8]>) -> String {
    store_file(contents).1
}

fn store_file(contents: impl AsRef<[u8]>) -> (String, String) {
    let name: String = rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    let file = vault_dir().join(&name);
    std::fs::write(&file, contents).expect("write");

    (name, String::from(file.to_string_lossy()))
}

fn vault_dir() -> PathBuf {
    let particle_id = get_call_parameters().particle.id;
    let token = get_call_parameters().particle.token;
    let path = format!("{}-{}", particle_id, token);
    let vault = Path::new("/tmp").join("vault").join(path);

    vault
}
