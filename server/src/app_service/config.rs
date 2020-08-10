/*
 * Copyright 2020 Fluence Labs Limited
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

use serde::Deserialize;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct AppServicesConfig {
    /// Path of the blueprint directory containing blueprints and wasm modules
    pub blueprint_dir: PathBuf,
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub service_envs: Vec<String>,
    pub services_workdir: PathBuf,
}

impl AppServicesConfig {
    pub fn new<A: ?Sized + AsRef<OsStr>, B: ?Sized + AsRef<OsStr>>(
        blueprint_dir: &A,
        service_envs: Vec<String>,
        workdir: &B,
    ) -> Self {
        Self {
            blueprint_dir: PathBuf::from(blueprint_dir),
            service_envs,
            services_workdir: PathBuf::from(workdir),
        }
    }
}
