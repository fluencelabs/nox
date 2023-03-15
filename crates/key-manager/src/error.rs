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

use libp2p::PeerId;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyManagerError {
    #[error("Failed to persist keypair: RSA is not supported")]
    CannotExtractRSASecretKey,
    #[error("Error reading persisted keypair from {path:?}: {err}")]
    ReadPersistedKeypair {
        path: PathBuf,
        #[source]
        err: std::io::Error,
    },
    #[error("Error deserializing persisted keypair from {path:?}: {err}")]
    DeserializePersistedKeypair {
        path: PathBuf,
        #[source]
        err: toml_edit::de::Error,
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
    #[error("Error removing persisted keypair {path:?}: {err}")]
    RemoveErrorPersistedKeypair {
        path: PathBuf,
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
    #[error("Worker for {deal_id} already exists")]
    WorkerAlreadyExists { deal_id: String },
    #[error("Worker for deal_id {0} not found")]
    WorkerNotFoundByDeal(String),
    #[error("Worker {0} not found")]
    WorkerNotFound(PeerId),
}
