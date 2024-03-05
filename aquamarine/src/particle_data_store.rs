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

use std::borrow::Cow;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use avm_server::avm_runner::RawAVMOutcome;
use avm_server::{AnomalyData, CallResults, ParticleParameters};
use fluence_libp2p::PeerId;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use thiserror::Error;
use tracing::instrument;

use now_millis::now_ms;
use particle_execution::{ParticleVault, VaultError};

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

    pub fn data_file(&self, particle_id: &str, current_peer_id: &str, signature: &[u8]) -> PathBuf {
        let key = store_key_from_components(particle_id, current_peer_id, signature);
        self.particle_data_store.join(key)
    }

    /// Returns $ANOMALY_DATA_STORE/$particle_id/$timestamp
    pub fn anomaly_dir(
        &self,
        particle_id: &str,
        current_peer_id: &str,
        signature: &[u8],
    ) -> PathBuf {
        let key = store_key_from_components(particle_id, current_peer_id, signature);
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

impl ParticleDataStore {
    pub async fn initialize(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.particle_data_store)
            .await
            .map_err(DataStoreError::CreateDataStore)?;

        self.vault.initialize().await?;

        Ok(())
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn store_data(
        &self,
        data: &[u8],
        particle_id: &str,
        current_peer_id: &str,
        signature: &[u8],
    ) -> Result<()> {
        tracing::trace!(target: "particle_reap", particle_id = particle_id, "Storing data for particle");
        let data_path = self.data_file(particle_id, current_peer_id, signature);
        tokio::fs::write(&data_path, data)
            .await
            .map_err(|err| DataStoreError::StoreData(err, data_path))?;

        Ok(())
    }

    #[instrument(level = tracing::Level::INFO)]
    pub async fn read_data(
        &self,
        particle_id: &str,
        current_peer_id: &str,
        signature: &[u8],
    ) -> Result<Vec<u8>> {
        let data_path = self.data_file(particle_id, current_peer_id, signature);
        let data = tokio::fs::read(&data_path).await.unwrap_or_default();
        Ok(data)
    }

    pub async fn batch_cleanup_data(&self, cleanup_keys: Vec<(String, PeerId, Vec<u8>, String)>) {
        let futures: FuturesUnordered<_> = cleanup_keys
            .into_iter()
            .map(
                |(particle_id, peer_id, signature, particle_token)| async move {
                    tracing::debug!(
                        target: "particle_reap",
                        particle_id = particle_id, worker_id = peer_id.to_base58(),
                        "Reaping particle's actor"
                    );

                    if let Err(err) = self
                        .cleanup_data(
                            particle_id.as_str(),
                            peer_id,
                            &signature,
                            particle_token.as_str(),
                        )
                        .await
                    {
                        tracing::warn!(
                            particle_id = particle_id,
                            "Error cleaning up after particle {:?}",
                            err
                        );
                    }
                },
            )
            .collect();
        let _results: Vec<_> = futures.collect().await;
    }

    async fn cleanup_data(
        &self,
        particle_id: &str,
        current_peer_id: PeerId,
        signature: &[u8],
        particle_token: &str,
    ) -> Result<()> {
        tracing::debug!(target: "particle_reap", particle_id = particle_id, "Cleaning up particle data for particle");
        let path = self.data_file(particle_id, &current_peer_id.to_base58(), signature);
        match tokio::fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            // ignore NotFound
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(DataStoreError::CleanupData(err)),
        }?;

        self.vault
            .cleanup(current_peer_id, particle_id, particle_token)
            .await?;

        Ok(())
    }

    fn detect_mem_limits_anomaly(&self, memory_delta: usize, outcome: &RawAVMOutcome) -> bool {
        memory_delta > MEMORY_DELTA_BYTES_THRESHOLD
            || outcome.soft_limits_triggering.are_limits_exceeded()
    }

    pub fn detect_anomaly(
        &self,
        execution_time: Duration,
        memory_delta: usize,
        outcome: &RawAVMOutcome,
    ) -> bool {
        execution_time > EXECUTION_TIME_THRESHOLD
            || outcome.ret_code != 0
            || self.detect_mem_limits_anomaly(memory_delta, outcome)
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn save_anomaly_data(
        &self,
        air_script: &str,
        current_data: &[u8],
        call_results: &CallResults,
        particle_parameters: &ParticleParameters<'_>,
        particle_signature: &[u8],
        outcome: &RawAVMOutcome,
        execution_time: Duration,
        memory_delta: usize,
    ) -> std::result::Result<(), DataStoreError> {
        let prev_data = self
            .read_data(
                &particle_parameters.particle_id,
                &particle_parameters.current_peer_id,
                particle_signature,
            )
            .await?;

        let ser_particle =
            serde_json::to_vec(particle_parameters).map_err(DataStoreError::SerializeAnomaly)?;
        let ser_call_results =
            serde_json::to_vec(call_results).map_err(DataStoreError::SerializeAnomaly)?;
        let ser_avm_outcome =
            serde_json::to_vec(outcome).map_err(DataStoreError::SerializeAnomaly)?;

        let anomaly_data = AnomalyData {
            air_script: Cow::Borrowed(air_script),
            particle: Cow::Owned(ser_particle),
            prev_data: Cow::Owned(prev_data),
            current_data: Cow::Borrowed(current_data),
            call_results: Cow::Owned(ser_call_results),
            avm_outcome: Cow::Owned(ser_avm_outcome),
            execution_time,
            memory_delta,
        };
        self.collect_anomaly_data(
            &particle_parameters.particle_id,
            &particle_parameters.current_peer_id,
            particle_signature,
            anomaly_data,
        )
        .await?;
        Ok(())
    }

    async fn collect_anomaly_data(
        &self,
        particle_id: &str,
        current_peer_id: &str,
        signature: &[u8],
        anomaly_data: AnomalyData<'_>,
    ) -> std::result::Result<(), DataStoreError> {
        let path = self.anomaly_dir(particle_id, current_peer_id, signature);
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(DataStoreError::CreateAnomalyDir)?;

        let file = path.join("data");
        let data = serde_json::to_vec(&anomaly_data).map_err(DataStoreError::SerializeAnomaly)?;
        tokio::fs::write(&file, data)
            .await
            .map_err(|err| DataStoreError::WriteAnomaly(err, file))?;

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
    #[error("error reading data from {1:?}")]
    ReadData(#[source] std::io::Error, PathBuf),
}

fn store_key_from_components(particle_id: &str, current_peer_id: &str, signature: &[u8]) -> String {
    format!(
        "particle_{particle_id}-peer_{current_peer_id}-sig_{}",
        format_signature(signature)
    )
}

fn format_signature(signature: &[u8]) -> String {
    bs58::encode(signature).into_string()
}

#[cfg(test)]
mod tests {
    use crate::ParticleDataStore;
    use avm_server::avm_runner::RawAVMOutcome;
    use avm_server::{CallRequests, SoftLimitsTriggering};
    use fluence_libp2p::PeerId;
    use std::path::PathBuf;
    use std::time::Duration;

    #[tokio::test]
    async fn test_initialize() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let particle_data_store = temp_dir.path().join("particle_data_store");
        let vault_dir = temp_dir.path().join("vault");
        let anomaly_data_store = temp_dir.path().join("anomaly_data_store");
        let particle_data_store_clone = particle_data_store.clone();

        let particle_data_store =
            ParticleDataStore::new(particle_data_store, vault_dir, anomaly_data_store);

        let result = particle_data_store.initialize().await;

        assert!(result.is_ok());
        assert!(particle_data_store_clone.exists());
    }

    #[tokio::test]
    async fn test_store_and_read_data() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let particle_data_store = temp_dir.path().join("particle_data_store");
        let vault_dir = temp_dir.path().join("vault");
        let anomaly_data_store = temp_dir.path().join("anomaly_data_store");

        let particle_data_store =
            ParticleDataStore::new(particle_data_store, vault_dir, anomaly_data_store);
        particle_data_store
            .initialize()
            .await
            .expect("Failed to initialize");

        let particle_id = "test_particle";
        let current_peer_id = "test_peer";
        let signature: &[u8] = &[0];
        let data = b"test_data";

        particle_data_store
            .store_data(data, particle_id, current_peer_id, signature)
            .await
            .expect("Failed to store data");
        let read_result = particle_data_store
            .read_data(particle_id, current_peer_id, signature)
            .await;

        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), data);
    }

    #[tokio::test]
    async fn test_detect_anomaly() {
        let particle_data_store = ParticleDataStore::new(
            PathBuf::from("dummy"),
            PathBuf::from("dummy"),
            PathBuf::from("dummy"),
        );

        let execution_time_below_threshold = Duration::from_millis(400);
        let execution_time_above_threshold = Duration::from_millis(600);
        let memory_delta_below_threshold = 5 * bytesize::MB as usize;
        let memory_delta_above_threshold = 15 * bytesize::MB as usize;
        let soft_limits_triggering = <_>::default();
        let outcome_success = RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data: vec![],
            call_requests: CallRequests::new(),
            next_peer_pks: vec![],
            soft_limits_triggering,
        };
        let outcome_failure = RawAVMOutcome {
            ret_code: 1,
            error_message: "".to_string(),
            data: vec![],
            call_requests: CallRequests::new(),
            next_peer_pks: vec![],
            soft_limits_triggering,
        };

        let anomaly_below_threshold = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome_success,
        );

        assert!(!anomaly_below_threshold);

        let anomaly_above_threshold = particle_data_store.detect_anomaly(
            execution_time_above_threshold,
            memory_delta_above_threshold,
            &outcome_failure,
        );

        assert!(anomaly_above_threshold);

        let anomaly_below_air_size_limit = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome_success,
        );

        assert!(!anomaly_below_air_size_limit);

        let soft_limits_triggering = SoftLimitsTriggering::new(true, false, false);
        let outcome = RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data: vec![],
            call_requests: CallRequests::new(),
            next_peer_pks: vec![],
            soft_limits_triggering,
        };
        let anomaly_above_air_size_limit = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome,
        );

        assert!(anomaly_above_air_size_limit);

        let anomaly_below_particle_size_limit = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome_success,
        );

        assert!(!anomaly_below_particle_size_limit);

        let soft_limits_triggering = SoftLimitsTriggering::new(false, true, false);
        let outcome = RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data: vec![],
            call_requests: CallRequests::new(),
            next_peer_pks: vec![],
            soft_limits_triggering,
        };
        let anomaly_above_particle_size_limit = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome,
        );

        assert!(anomaly_above_particle_size_limit);

        let soft_limits_triggering = SoftLimitsTriggering::new(false, false, true);
        let outcome = RawAVMOutcome {
            ret_code: 0,
            error_message: "".to_string(),
            data: vec![],
            call_requests: CallRequests::new(),
            next_peer_pks: vec![],
            soft_limits_triggering,
        };
        let anomaly_below_call_result_size_limit = particle_data_store.detect_anomaly(
            execution_time_below_threshold,
            memory_delta_below_threshold,
            &outcome,
        );

        assert!(anomaly_below_call_result_size_limit);

        // let anomaly_call_result_size =
    }

    #[tokio::test]
    async fn test_cleanup_data() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_dir_path = temp_dir.path();
        let particle_data_store = ParticleDataStore::new(
            temp_dir_path.join("particle_data_store"),
            temp_dir_path.join("vault"),
            temp_dir_path.join("anomaly_data_store"),
        );
        particle_data_store
            .initialize()
            .await
            .expect("Failed to initialize");

        let particle_id = "test_particle";
        let particle_token = "test_token";
        let current_peer_id = PeerId::random();
        let current_peer_id_str = current_peer_id.to_base58();
        let signature: &[u8] = &[];
        let data = b"test_data";

        particle_data_store
            .store_data(data, particle_id, &current_peer_id_str, signature)
            .await
            .expect("Failed to store data");

        let data_file_path =
            particle_data_store.data_file(particle_id, &current_peer_id_str, signature);
        let vault_path = particle_data_store.vault.real_particle_vault(
            current_peer_id,
            particle_id,
            particle_token,
        );
        tokio::fs::create_dir_all(&vault_path)
            .await
            .expect("Failed to create vault dir");
        assert!(data_file_path.exists());
        assert!(vault_path.exists());

        let cleanup_result = particle_data_store
            .cleanup_data(particle_id, current_peer_id, signature, particle_token)
            .await;

        assert!(cleanup_result.is_ok());
        assert!(!data_file_path.exists());
        assert!(!vault_path.exists())
    }
}
