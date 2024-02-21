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
use crate::defaults::{avm_base_dir, default_base_dir, services_dir};

use air_interpreter_fs::air_interpreter_path;
use fs_utils::{canonicalize, create_dirs, to_abs_path};

use crate::{ephemeral_dir, persistent_dir};
use eyre::WrapErr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UnresolvedDirConfig {
    /// Parent directory for all other node's directory
    #[serde(default = "default_base_dir")]
    pub base_dir: PathBuf,

    /// Base directory for persistent resources
    pub persistent_base_dir: Option<PathBuf>,

    /// Base directory for ephemeral resources
    pub ephemeral_base_dir: Option<PathBuf>,

    /// Base directory for persistent resources needed by application services
    pub services_persistent_dir: Option<PathBuf>,

    /// Base directory for ephemeral resources needed by application services
    pub services_ephemeral_dir: Option<PathBuf>,

    /// Base directory for resources needed by application services
    pub avm_base_dir: Option<PathBuf>,

    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    pub air_interpreter_path: Option<PathBuf>,

    /// Path to spell service files (wasms, configs)
    pub spell_base_dir: Option<PathBuf>,

    /// Path to persisted worker keypairs
    pub keypairs_base_dir: Option<PathBuf>,

    /// Path to persisted workers
    pub workers_base_dir: Option<PathBuf>,

    /// Path to stored cc events
    pub cc_events_dir: Option<PathBuf>,

    /// Path to stored core_state
    pub core_state_path: Option<PathBuf>,
}

impl UnresolvedDirConfig {
    pub fn resolve(self) -> eyre::Result<ResolvedDirConfig> {
        let base = to_abs_path(self.base_dir);

        let ephemeral_base_dir = self.ephemeral_base_dir.unwrap_or(ephemeral_dir(&base));
        let persistent_base_dir = self.persistent_base_dir.unwrap_or(persistent_dir(&base));

        // ephemeral dirs
        let services_ephemeral_dir = self
            .services_ephemeral_dir
            .unwrap_or(services_dir(&ephemeral_base_dir));
        let avm_base_dir = self
            .avm_base_dir
            .unwrap_or(avm_base_dir(&ephemeral_base_dir));

        // persistent dirs
        let services_persistent_dir = self
            .services_persistent_dir
            .unwrap_or(services_dir(&persistent_base_dir));
        let air_interpreter_path = self
            .air_interpreter_path
            .unwrap_or(air_interpreter_path(&persistent_base_dir));
        let spell_base_dir = self
            .spell_base_dir
            .unwrap_or(persistent_base_dir.join("spell"));
        let keypairs_base_dir = self
            .keypairs_base_dir
            .unwrap_or(persistent_base_dir.join("keypairs"));
        let workers_base_dir = self
            .workers_base_dir
            .unwrap_or(persistent_base_dir.join("workers"));

        let cc_events_dir = self
            .cc_events_dir
            .unwrap_or(persistent_base_dir.join("cc_events"));
        let core_state_path = self
            .core_state_path
            .clone()
            .unwrap_or(persistent_base_dir.join("cores_state.toml"));

        create_dirs(&[
            &base,
            // ephemeral dirs
            &ephemeral_base_dir,
            &services_ephemeral_dir,
            &avm_base_dir,
            // persistent dirs
            &persistent_base_dir,
            &services_persistent_dir,
            &spell_base_dir,
            &keypairs_base_dir,
            &workers_base_dir,
            // other
            &cc_events_dir,
        ])
        .context("creating configured directories")?;

        let base_dir = canonicalize(base)?;
        // ephemeral dirs
        let avm_base_dir = canonicalize(avm_base_dir)?;
        let services_ephemeral_dir = canonicalize(services_ephemeral_dir)?;

        // persistent dirs
        let services_persistent_dir = canonicalize(services_persistent_dir)?;
        let spell_base_dir = canonicalize(spell_base_dir)?;
        let keypairs_base_dir = canonicalize(keypairs_base_dir)?;
        let workers_base_dir = canonicalize(workers_base_dir)?;

        let cc_events_dir = canonicalize(cc_events_dir)?;

        Ok(ResolvedDirConfig {
            base_dir,
            ephemeral_base_dir,
            persistent_base_dir,
            avm_base_dir,
            services_ephemeral_dir,
            services_persistent_dir,
            air_interpreter_path,
            spell_base_dir,
            keypairs_base_dir,
            workers_base_dir,
            cc_events_dir,
            core_state_path,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedDirConfig {
    pub base_dir: PathBuf,
    pub ephemeral_base_dir: PathBuf,
    pub persistent_base_dir: PathBuf,
    /// Directory where particle's prev_data is stored
    pub avm_base_dir: PathBuf,
    pub services_ephemeral_dir: PathBuf,
    pub services_persistent_dir: PathBuf,
    /// Directory where interpreter's WASM module is stored
    pub air_interpreter_path: PathBuf,
    pub spell_base_dir: PathBuf,
    pub keypairs_base_dir: PathBuf,
    pub workers_base_dir: PathBuf,
    pub cc_events_dir: PathBuf,
    pub core_state_path: PathBuf,
}
