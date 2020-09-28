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

use config::{abs_path, create_dirs};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ActorConfig {
    /// Working dir for steppers
    pub workdir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running steppers
    pub services_dir: PathBuf,
}

impl ActorConfig {
    pub fn new(base_dir: PathBuf) -> Result<Self, std::io::Error> {
        let base_dir = abs_path(base_dir);

        let this = Self {
            workdir: config::workdir(&base_dir),
            modules_dir: config::modules_dir(&base_dir),
            services_dir: config::services_dir(&base_dir),
        };

        this.create_dirs()?;

        Ok(this)
    }

    pub fn create_dirs(&self) -> Result<(), std::io::Error> {
        create_dirs(&[&self.workdir, &self.modules_dir, &self.services_dir])
    }
}
