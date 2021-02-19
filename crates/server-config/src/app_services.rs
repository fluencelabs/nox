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

use config_utils::create_dirs;

use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppServicesConfig {
    /// Path of the blueprint directory containing blueprints and wasm modules
    pub blueprint_dir: PathBuf,
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub service_envs: Vec<String>,
    /// Working dir for services
    pub workdir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running services
    pub services_dir: PathBuf,
    /// Path to file to store aliases
    pub aliases_path: PathBuf,
}

impl AppServicesConfig {
    pub fn new<A: ?Sized + AsRef<OsStr>>(
        base_dir: &A,
        service_envs: Vec<String>,
    ) -> Result<Self, std::io::Error> {
        // if cwd is available, make given path absolute
        let base_dir = PathBuf::from(base_dir);
        let base_dir = match std::env::current_dir().ok() {
            Some(c) => c.join(base_dir),
            None => base_dir,
        };

        let blueprint_dir = base_dir.join("blueprint");
        let workdir = base_dir.join("workdir");
        let modules_dir = base_dir.join("modules");
        let services_dir = base_dir.join("services");
        let aliases_path = base_dir.join("aliases.list");

        let this = Self {
            blueprint_dir,
            workdir,
            modules_dir,
            services_dir,
            service_envs,
            aliases_path,
        };

        this.create_dirs()?;

        Ok(this)
    }

    pub fn create_dirs(&self) -> Result<(), std::io::Error> {
        create_dirs(&[
            &self.blueprint_dir,
            &self.workdir,
            &self.modules_dir,
            &self.services_dir,
        ])
    }
}
