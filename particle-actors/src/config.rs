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

use config_utils::{create_dirs, to_abs_path};
use libp2p::PeerId;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VmPoolConfig {
    pub current_peer_id: PeerId,
    /// Working dir for steppers
    pub workdir: PathBuf,
    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    pub air_interpreter: PathBuf,
    /// Dir to persist info about running steppers
    pub services_dir: PathBuf,
    /// Dir for stepper to persist particle data to merge it
    pub particles_dir: PathBuf,
    /// Number of VMs to create
    pub pool_size: usize,
}

impl VmPoolConfig {
    pub fn new(
        current_peer_id: PeerId,
        base_dir: PathBuf,
        air_interpreter: PathBuf,
        pool_size: usize,
    ) -> Result<Self, std::io::Error> {
        let base_dir = to_abs_path(base_dir);

        let this = Self {
            current_peer_id,
            workdir: config_utils::workdir(&base_dir),
            services_dir: config_utils::services_dir(&base_dir),
            particles_dir: config_utils::particles_dir(&base_dir),
            air_interpreter,
            pool_size,
        };

        this.create_dirs()?;

        Ok(this)
    }

    pub fn create_dirs(&self) -> Result<(), std::io::Error> {
        create_dirs(&[&self.workdir, &self.services_dir])
    }
}

#[cfg(test)]
impl Default for VmPoolConfig {
    fn default() -> Self {
        Self {
            current_peer_id: fluence_libp2p::RandomPeerId::random(),
            workdir: <_>::default(),
            air_interpreter: <_>::default(),
            services_dir: <_>::default(),
            particles_dir: <_>::default(),
            pool_size: 0,
        }
    }
}
