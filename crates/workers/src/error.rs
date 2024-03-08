/*
 * Copyright 2022 Fluence Labs Limited
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

use core_manager::errors::AcquireError;
use libp2p::PeerId;
use std::path::PathBuf;
use thiserror::Error;
use types::peer_scope::WorkerId;
use types::DealId;

#[derive(Debug, Error)]
pub enum KeyStorageError {
    #[error("Failed to persist keypair: RSA is not supported")]
    CannotExtractRSASecretKey,
    #[error("Error reading persisted keypair from {path:?}: {err}")]
    ReadPersistedKeypair {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error reading list of directory from {path:?}: {err}")]
    DirectoryListError {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error deserializing persisted keypair from {path:?}: {err}")]
    DeserializePersistedKeypair {
        path: PathBuf,
        #[source]
        err: toml::de::Error,
    },
    #[error("Failed to decode keypair {path}: {err}")]
    PersistedKeypairDecodingError {
        path: PathBuf,
        #[source]
        err: fluence_keypair::error::DecodingError,
    },
    #[error("Invalid key format {path}: {err}")]
    PersistedKeypairInvalidKeyFormat {
        path: PathBuf,
        #[source]
        err: fluence_keypair::error::Error,
    },
    #[error("Error serializing persisted keypair: {err}")]
    SerializePersistedKeypair {
        #[source]
        err: toml::ser::Error,
    },
    #[error("Error writing persisted keypair to {path:?}: {err}")]
    WriteErrorPersistedKeypair {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error removing persisted keypair {path:?} for worker {worker_id}: {err}")]
    RemoveErrorPersistedKeypair {
        path: PathBuf,
        worker_id: WorkerId,
        #[source]
        err: std::io::Error,
    },
    #[error("Error creating directory for persisted keypairs {path:?}: {err}")]
    CreateKeypairsDir {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },

    #[error("Keypair for peer_id {0} not found")]
    KeypairNotFound(PeerId),
}

#[derive(Debug, Error)]
pub enum WorkersError {
    #[error("Error creating directory for persisted workers {path:?}: {err}")]
    CreateWorkersDir {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error creating key pair for worker: {err}")]
    CreateWorkerKeyPair {
        #[source]
        err: KeyStorageError,
    },
    #[error("Error removing key pair for worker: {err}")]
    RemoveWorkerKeyPair {
        #[source]
        err: KeyStorageError,
    },
    #[error("Error reading persisted worker from {path:?}: {err}")]
    ReadPersistedWorker {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error deserializing persisted worker from {path:?}: {err}")]
    DeserializePersistedWorker {
        path: PathBuf,
        #[source]
        err: toml::de::Error,
    },
    #[error("Worker for {deal_id} already exists")]
    WorkerAlreadyExists { deal_id: DealId },
    #[error("Worker for deal_id {0} not found")]
    WorkerNotFoundByDeal(DealId),
    #[error("Worker {0} not found")]
    WorkerNotFound(WorkerId),
    #[error("Error serializing persisted worker: {err}")]
    SerializePersistedWorker {
        #[source]
        err: toml::ser::Error,
    },
    #[error("Error writing persisted worker to {path:?}: {err}")]
    WriteErrorPersistedWorker {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error removing persisted worker {path:?} for worker {worker_id}: {err}")]
    RemoveErrorPersistedWorker {
        path: PathBuf,
        worker_id: WorkerId,
        #[source]
        err: std::io::Error,
    },
    #[error("Keypair for peer_id {0} not found")]
    KeypairNotFound(PeerId),
    #[error("Failed to create runtime for worker {worker_id}: {err}")]
    CreateRuntime {
        worker_id: WorkerId,
        #[source]
        err: std::io::Error,
    },
    #[error("Failed to allocate cores for {worker_id}: {err}")]
    FailedToAssignCores {
        worker_id: WorkerId,
        #[source]
        err: AcquireError,
    },
}
