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

use avm_server::DataStore;
use thiserror::Error;

use fs_utils::{create_dir, remove_dir};
use DataStoreError::{CleanupData, CreateDataStore, StoreData};

use crate::particle_vault::VaultError;
use crate::ParticleVault;

type Result<T> = std::result::Result<T, DataStoreError>;

#[derive(Debug, Clone)]
pub struct ParticleDataStore {
    pub particle_data_store: PathBuf,
    pub vault: ParticleVault,
}

impl ParticleDataStore {
    pub fn new(particle_data_store: PathBuf, vault_dir: PathBuf) -> Self {
        Self {
            particle_data_store,
            vault: ParticleVault::new(vault_dir),
        }
    }

    pub fn data_file(&self, key: &str) -> PathBuf {
        self.particle_data_store.join(key)
    }

    pub fn create_particle_vault(&self, key: &str) -> Result<()> {
        self.vault.create(key)?;

        Ok(())
    }
}

impl DataStore<DataStoreError> for ParticleDataStore {
    fn initialize(&mut self) -> Result<()> {
        create_dir(&self.particle_data_store).map_err(CreateDataStore)?;

        self.vault.initialize()?;

        Ok(())
    }

    fn store_data(&mut self, data: &[u8], key: &str) -> Result<()> {
        let data_path = self.data_file(&key);
        std::fs::write(&data_path, data).map_err(|err| StoreData(err, data_path))?;

        Ok(())
    }

    fn read_data(&mut self, key: &str) -> Result<Vec<u8>> {
        let data_path = self.data_file(&key);
        let data = std::fs::read(&data_path).unwrap_or_default();

        Ok(data)
    }

    fn cleanup_data(&mut self, key: &str) -> Result<()> {
        remove_dir(&self.data_file(key)).map_err(CleanupData)?;
        self.vault.cleanup(key)?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DataStoreError {
    #[error("error creating particle_data_store")]
    CreateDataStore(#[source] std::io::Error),
    #[error(transparent)]
    VaultError(#[from] VaultError),
    #[error("error writing data to {1:?}")]
    StoreData(#[source] std::io::Error, PathBuf),
    #[error("error cleaning up data")]
    CleanupData(#[source] std::io::Error),
}
