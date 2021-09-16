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

use anyhow::Result;
use avm_server::DataStore;

use fs_utils::{create_dir, create_dirs, remove_dir};

pub struct ParticleDataStore {
    pub particle_data_store: PathBuf,
    pub vault_dir: PathBuf,
}

impl ParticleDataStore {
    pub fn data_file(&self, key: &str) -> PathBuf {
        self.particle_data_store.join(key)
    }

    pub fn particle_vault(&self, key: &str) -> PathBuf {
        self.vault_dir.join(key)
    }

    pub fn create_particle_vault(&self, key: &str) -> Result<()> {
        let path = self.particle_vault(key);
        create_dir(path).wrap_err("error creating particle vault dir")?;

        Ok(())
    }
}

impl DataStore for ParticleDataStore {
    fn initialize(&mut self) -> Result<()> {
        create_dir(&self.particle_data_store).wrap_err("error creating particle_data_store")?;
        create_dir(&self.vault_dir).wrap_err("error creating vault_dir")?;

        Ok(())
    }

    fn store_data(&mut self, data: &[u8], key: &str) -> Result<()> {
        let data_path = self.data_file(&key);
        std::fs::write(&data_path, data).wrap_err_with(|| format!("error writing data to {:?}", data_path))?;

        Ok(())
    }

    fn read_data(&mut self, key: &str) -> Result<Vec<u8>> {
        let data_path = self.data_file(&key);
        let data = std::fs::read(&data_path).wrap_err_with(|| format!("error reading from {:?}", data_path))?;

        Ok(data)
    }

    fn cleanup_data(&mut self, key: &str) -> Result<()> {
        remove_dirs(&[&self.particle_vault(key), &self.data_file(key)])
    }
}
