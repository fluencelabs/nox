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

use particle_modules::{AddBlueprint, NamedModuleConfig};

#[derive(Debug)]
pub struct ScheduledScript {
    pub name: String,
    pub data: String,
    pub interval_sec: u64,
}

#[derive(Debug)]
pub struct Module {
    // .wasm data
    pub data: Vec<u8>,
    // parsed json module config
    pub config: NamedModuleConfig,
}

#[derive(Debug)]
pub struct Builtin {
    // builtin alias
    pub name: String,
    // list of dependencies
    pub modules: Vec<Module>,
    pub blueprint: AddBlueprint,
    pub blueprint_id: String,
    pub on_start_script: Option<String>,
    pub on_start_data: Option<String>,
    pub scheduled_scripts: Vec<ScheduledScript>,
}
