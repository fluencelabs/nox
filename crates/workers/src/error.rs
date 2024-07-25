/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use core_distributor::errors::AcquireError;
use libp2p::PeerId;
use std::path::PathBuf;
use thiserror::Error;
use types::peer_scope::WorkerId;
use types::DealId;
use vm_utils::VMUtilsError;

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
        err: toml_edit::ser::Error,
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
        err: toml_edit::ser::Error,
    },
    #[error("Error writing persisted worker to {path:?}: {err}")]
    WriteErrorPersistedWorker {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error creation of a persisted worker directory {path} for worker {worker_id}: {err}")]
    WorkerStorageDirectory {
        path: PathBuf,
        worker_id: WorkerId,
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
    #[error("Failed to notify subsystem {worker_id}")]
    FailedToNotifySubsystem { worker_id: WorkerId },
    #[error("Failed to remove VM {worker_id}")]
    FailedToCreateVM {
        worker_id: WorkerId,
        err: VMUtilsError,
    },
    #[error("Failed to remove VM {worker_id}")]
    FailedToRemoveVM {
        worker_id: WorkerId,
        err: VMUtilsError,
    },
    #[error("This feature is disabled")]
    FeatureDisabled,
    #[error("VM image {image} isn't file")]
    VMImageNotFile { image: PathBuf },
    #[error("Failed to copy {image} to the worker storage: {err}")]
    FailedToCopyVMImage { image: PathBuf, err: std::io::Error },
}
