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

use marine_rs_sdk::get_call_parameters;
use marine_rs_sdk::marine;
use marine_rs_sdk::module_manifest;
use marine_rs_sdk::SecurityTetraplet;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Mutex;

module_manifest!();

static KV: OnceCell<Mutex<HashMap<String, String>>> = OnceCell::new();

pub fn main() {
    KV.set(<_>::default()).ok();
}

#[marine]
pub fn get_tetraplets(_: String) -> Vec<Vec<SecurityTetraplet>> {
    get_call_parameters().tetraplets
}

#[marine]
pub fn not(b: bool) -> bool {
    !b
}

#[marine]
pub fn store(key: String, value: String) {
    KV.get().unwrap().lock().unwrap().insert(key, value);
}

#[marine]
pub fn delete(key: String) {
    KV.get().unwrap().lock().unwrap().remove(&key);
}
