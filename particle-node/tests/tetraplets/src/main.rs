#![feature(once_cell)]
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
