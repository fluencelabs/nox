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
use crate::defaults::{avm_base_dir, default_base_dir, services_base_dir};

use air_interpreter_fs::air_interpreter_path;
use fs_utils::{canonicalize, create_dirs, to_abs_path};

use eyre::WrapErr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UnresolvedDirConfig {
    /// Parent directory for all other node's directory
    #[serde(default = "default_base_dir")]
    pub base_dir: PathBuf,

    /// Base directory for resources needed by application services
    pub services_base_dir: Option<PathBuf>,

    /// Base directory for resources needed by application services
    pub avm_base_dir: Option<PathBuf>,

    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    pub air_interpreter_path: Option<PathBuf>,

    /// Path to spell service files (wasms, configs)
    pub spell_base_dir: Option<PathBuf>,

    pub keypairs_base_dir: Option<PathBuf>,
}

impl UnresolvedDirConfig {
    pub fn resolve(self) -> eyre::Result<ResolvedDirConfig> {
        let base = to_abs_path(self.base_dir);

        let services_base_dir = self.services_base_dir.unwrap_or(services_base_dir(&base));
        let avm_base_dir = self.avm_base_dir.unwrap_or(avm_base_dir(&base));
        let air_interpreter_path = self
            .air_interpreter_path
            .unwrap_or(air_interpreter_path(&base));
        let spell_base_dir = self.spell_base_dir.unwrap_or(base.join("spell"));
        let keypairs_base_dir = self.keypairs_base_dir.unwrap_or(base.join("keypairs"));

        create_dirs(&[
            &base,
            &services_base_dir,
            &avm_base_dir,
            &spell_base_dir,
            &keypairs_base_dir,
        ])
        .context("creating configured directories")?;

        let base = canonicalize(base)?;
        let services_base_dir = canonicalize(services_base_dir)?;
        let avm_base_dir = canonicalize(avm_base_dir)?;
        let spell_base_dir = canonicalize(spell_base_dir)?;
        let keypairs_base_dir = canonicalize(keypairs_base_dir)?;

        Ok(ResolvedDirConfig {
            base_dir: base,
            services_base_dir,
            avm_base_dir,
            air_interpreter_path,
            spell_base_dir,
            keypairs_base_dir,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedDirConfig {
    pub base_dir: PathBuf,
    pub services_base_dir: PathBuf,
    /// Directory where particle's prev_data is stored
    pub avm_base_dir: PathBuf,
    /// Directory where interpreter's WASM module is stored
    pub air_interpreter_path: PathBuf,
    pub spell_base_dir: PathBuf,
    pub keypairs_base_dir: PathBuf,
}
