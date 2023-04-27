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

use std::path::{Path, PathBuf};
use std::time::Duration;

use avm_server::avm_runner::RawAVMOutcome;
use avm_server::{AnomalyData, DataStore};
use thiserror::Error;

use fs_utils::{create_dir, remove_file};
use now_millis::now_ms;
use particle_execution::{ParticleVault, VaultError};
use DataStoreError::{CleanupData, CreateDataStore, StoreData};

use crate::DataStoreError::{SerializeAnomaly, WriteAnomaly};

type Result<T> = std::result::Result<T, DataStoreError>;

#[derive(Debug, Clone)]
pub struct ParticleDataStore {
    pub particle_data_store: PathBuf,
    pub vault: ParticleVault,
    pub anomaly_data_store: PathBuf,
}

impl ParticleDataStore {
    pub fn new(
        particle_data_store: PathBuf,
        vault_dir: PathBuf,
        anomaly_data_store: PathBuf,
    ) -> Self {
        Self {
            particle_data_store,
            vault: ParticleVault::new(vault_dir),
            anomaly_data_store,
        }
    }

    pub fn data_file(&self, particle_id: &str, current_peer_id: &str) -> PathBuf {
        let key = store_key_from_components(particle_id, current_peer_id);
        self.particle_data_store.join(key)
    }

    /// Returns $ANOMALY_DATA_STORE/$particle_id/$timestamp
    pub fn anomaly_dir(&self, particle_id: &str, current_peer_id: &str) -> PathBuf {
        let key = store_key_from_components(particle_id, current_peer_id);
        [
            self.anomaly_data_store.as_path(),
            Path::new(&key),
            Path::new(&now_ms().to_string()),
        ]
        .iter()
        .collect()
    }
}

const EXECUTION_TIME_THRESHOLD: Duration = Duration::from_millis(500);
const MEMORY_DELTA_BYTES_THRESHOLD: usize = 10 * bytesize::MB as usize;

impl DataStore for ParticleDataStore {
    type Error = DataStoreError;

    fn initialize(&mut self) -> Result<()> {
        create_dir(&self.particle_data_store).map_err(CreateDataStore)?;

        self.vault.initialize()?;

        Ok(())
    }

    fn store_data(&mut self, data: &[u8], particle_id: &str, current_peer_id: &str) -> Result<()> {
        tracing::debug!(target: "particle_reap", particle_id = particle_id, "Storing data for particle");
        let data_path = self.data_file(particle_id, current_peer_id);
        std::fs::write(&data_path, data).map_err(|err| StoreData(err, data_path))?;

        Ok(())
    }

    fn read_data(&mut self, particle_id: &str, current_peer_id: &str) -> Result<Vec<u8>> {
        let data_path = self.data_file(particle_id, current_peer_id);
        let data = std::fs::read(data_path).unwrap_or_default();
        Ok(data)
    }

    fn cleanup_data(&mut self, particle_id: &str, current_peer_id: &str) -> Result<()> {
        tracing::debug!(target: "particle_reap", particle_id = particle_id, "Cleaning up particle data for particle");
        remove_file(&self.data_file(particle_id, current_peer_id)).map_err(CleanupData)?;
        self.vault.cleanup(particle_id)?;

        Ok(())
    }

    fn detect_anomaly(
        &self,
        execution_time: Duration,
        memory_delta: usize,
        outcome: &RawAVMOutcome,
    ) -> bool {
        execution_time > EXECUTION_TIME_THRESHOLD
            || memory_delta > MEMORY_DELTA_BYTES_THRESHOLD
            || outcome.ret_code != 0
    }

    fn collect_anomaly_data(
        &mut self,
        particle_id: &str,
        current_peer_id: &str,
        anomaly_data: AnomalyData<'_>,
    ) -> std::result::Result<(), Self::Error> {
        let path = self.anomaly_dir(particle_id, current_peer_id);
        create_dir(&path).map_err(DataStoreError::CreateAnomalyDir)?;

        let file = path.join("data");
        let data = serde_json::to_vec(&anomaly_data).map_err(SerializeAnomaly)?;
        std::fs::write(&file, data).map_err(|err| WriteAnomaly(err, file))?;

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
    #[error("error creating anomaly dir")]
    CreateAnomalyDir(#[source] std::io::Error),
    #[error("error writing anomaly data to {1:?}")]
    WriteAnomaly(#[source] std::io::Error, PathBuf),
    #[error("error serializing anomaly data")]
    SerializeAnomaly(#[source] serde_json::error::Error),
}

fn store_key_from_components(particle_id: &str, current_peer_id: &str) -> String {
    format!("particle_{particle_id}-peer_{current_peer_id}")
}
