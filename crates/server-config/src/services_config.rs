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
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServicesConfig {
    /// Peer id of the current node
    pub local_peer_id: PeerId,
    /// Path of the blueprint directory containing blueprints and wasm modules
    pub blueprint_dir: PathBuf,
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub envs: HashMap<Vec<u8>, Vec<u8>>,
    /// Working dir for services
    pub workdir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running services
    pub services_dir: PathBuf,
}

impl ServicesConfig {
    pub fn new(
        local_peer_id: PeerId,
        base_dir: PathBuf,
        envs: HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Self, std::io::Error> {
        let base_dir = to_abs_path(base_dir);

        let this = Self {
            local_peer_id,
            blueprint_dir: config_utils::blueprint_dir(&base_dir),
            workdir: config_utils::workdir(&base_dir),
            modules_dir: config_utils::modules_dir(&base_dir),
            services_dir: config_utils::services_dir(&base_dir),
            envs,
        };

        create_dirs(&[
            &this.blueprint_dir,
            &this.workdir,
            &this.modules_dir,
            &this.services_dir,
        ])?;

        Ok(this)
    }
}
