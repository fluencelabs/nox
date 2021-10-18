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

use std::path::PathBuf;

use thiserror::Error;

use crate::particle_vault::VaultError::{CleanupVault, CreateVault, InitializeVault};
use fs_utils::{create_dir, remove_dir};

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

    pub fn cleanup(&self, particle_id: &str) -> Result<(), VaultError> {
        remove_dir(&self.particle_vault(particle_id)).map_err(CleanupVault)?;

        Ok(())
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
}
