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

use std::path;
use std::path::{Path, PathBuf};

use thiserror::Error;

use fs_utils::{create_dir, remove_dir};

use crate::VaultError::WrongVault;
use VaultError::{CleanupVault, CreateVault, InitializeVault};

pub const VIRTUAL_PARTICLE_VAULT_PREFIX: &str = "/tmp/vault";

#[derive(Debug, Clone)]
pub struct ParticleVault {
    pub vault_dir: PathBuf,
}

impl ParticleVault {
    pub fn new(vault_dir: PathBuf) -> Self {
        Self { vault_dir }
    }

    pub fn particle_vault(&self, key: &str) -> PathBuf {
        self.vault_dir.join(key)
    }

    pub fn initialize(&self) -> Result<(), VaultError> {
        create_dir(&self.vault_dir).map_err(InitializeVault)?;

        Ok(())
    }

    pub fn create(&self, particle_id: &str) -> Result<(), VaultError> {
        let path = self.particle_vault(particle_id);
        create_dir(path).map_err(CreateVault)?;

        Ok(())
    }

    pub fn put(
        &self,
        particle_id: &str,
        path: &Path,
        payload: &str,
    ) -> Result<PathBuf, VaultError> {
        let vault_dir = self.particle_vault(particle_id);
        let real_path = vault_dir.join(path);
        if let Some(parent_path) = real_path.parent() {
            create_dir(parent_path).map_err(CreateVault)?;
        }

        std::fs::write(real_path.clone(), payload.as_bytes())
            .map_err(|e| VaultError::WriteVault(e, path.to_path_buf()))?;

        self.to_virtual_path(&real_path, particle_id)
    }

    pub fn cat(&self, particle_id: &str, virtual_path: &Path) -> Result<String, VaultError> {
        let real_path = self.to_real_path(&virtual_path, particle_id)?;

        let contents = std::fs::read_to_string(real_path)
            .map_err(|e| VaultError::ReadVault(e, virtual_path.to_path_buf()))?;

        Ok(contents)
    }

    pub fn cleanup(&self, particle_id: &str) -> Result<(), VaultError> {
        remove_dir(&self.particle_vault(particle_id)).map_err(CleanupVault)?;

        Ok(())
    }

    fn to_virtual_path(&self, path: &Path, particle_id: &str) -> Result<PathBuf, VaultError> {
        let virtual_prefix = path::Path::new(VIRTUAL_PARTICLE_VAULT_PREFIX).join(particle_id);
        let real_prefix = self.vault_dir.join(particle_id);
        let rest = path
            .strip_prefix(&real_prefix)
            .map_err(|e| WrongVault(Some(e), path.to_path_buf(), real_prefix))?;

        Ok(virtual_prefix.join(rest))
    }

    fn to_real_path(&self, path: &Path, particle_id: &str) -> Result<PathBuf, VaultError> {
        let virtual_prefix = path::Path::new(VIRTUAL_PARTICLE_VAULT_PREFIX).join(particle_id);
        let real_prefix = self.vault_dir.join(particle_id);

        let rest = path
            .strip_prefix(&virtual_prefix)
            .map_err(|e| WrongVault(Some(e), path.to_path_buf(), virtual_prefix.clone()))?;
        let real_path = real_prefix.join(rest);
        let resolved_path = real_path
            .canonicalize()
            .map_err(|e| VaultError::NotFound(e, path.to_path_buf()))?;
        // Check again after normalization that the path leads to the real particle vault
        if resolved_path.starts_with(&real_prefix) {
            Ok(resolved_path)
        } else {
            Err(WrongVault(None, resolved_path, real_prefix))
        }
    }
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("error creating vault_dir")]
    InitializeVault(#[source] std::io::Error),
    #[error("error creating particle vault")]
    CreateVault(#[source] std::io::Error),
    #[error("error cleaning up particle vault")]
    CleanupVault(#[source] std::io::Error),
    #[error("Incorrect vault path `{1}`: doesn't belong to vault (`{2}`)")]
    WrongVault(#[source] Option<path::StripPrefixError>, PathBuf, PathBuf),
    #[error("Incorrect vault  path `{1}`: doesn't exist")]
    NotFound(#[source] std::io::Error, PathBuf),
    #[error("Read vault failed for `{1}`: {0}")]
    ReadVault(#[source] std::io::Error, PathBuf),
    #[error("Write vault failed for `{1}`: {0}")]
    WriteVault(#[source] std::io::Error, PathBuf),
}
