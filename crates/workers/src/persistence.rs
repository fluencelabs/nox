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

use crate::error::KeyStorageError::{
    CannotExtractRSASecretKey, SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use crate::error::{KeyStorageError, WorkersError};
use crate::workers::WorkerInfo;
use crate::KeyStorageError::RemoveErrorPersistedKeypair;
use fluence_keypair::KeyPair;
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use types::WorkerId;

pub const fn default_bool<const V: bool>() -> bool {
    V
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedKeypair {
    pub private_key_bytes: Vec<u8>,
    pub key_format: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedWorker {
    pub worker_id: WorkerId,
    #[serde(with = "peerid_serializer")]
    pub creator: PeerId,
    #[serde(default)]
    pub deal_id: String,
    #[serde(default = "default_bool::<true>")]
    pub active: bool,
    pub cu_count: usize,
}

impl From<PersistedWorker> for WorkerInfo {
    fn from(val: PersistedWorker) -> Self {
        WorkerInfo {
            deal_id: val.deal_id,
            creator: val.creator,
            active: RwLock::new(val.active),
            cu_count: val.cu_count,
        }
    }
}

impl TryFrom<&KeyPair> for PersistedKeypair {
    type Error = KeyStorageError;

    fn try_from(keypair: &KeyPair) -> Result<Self, Self::Error> {
        Ok(Self {
            private_key_bytes: keypair.secret().map_err(|_| CannotExtractRSASecretKey)?,
            key_format: keypair.public().get_key_format().into(),
        })
    }
}

pub fn keypair_file_name(worker_id: WorkerId) -> String {
    format!("{}_keypair.toml", worker_id)
}

pub(crate) fn worker_file_name(worker_id: WorkerId) -> String {
    format!("{}_info.toml", worker_id)
}

fn is_keypair(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_keypair.toml"))
}

pub(crate) fn is_worker(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_info.toml"))
}

/// Persist keypair info to disk, so it is recreated after restart
pub(crate) async fn persist_keypair(
    keypairs_dir: &Path,
    worker_id: WorkerId,
    persisted_keypair: PersistedKeypair,
) -> Result<(), KeyStorageError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    let bytes =
        toml::to_vec(&persisted_keypair).map_err(|err| SerializePersistedKeypair { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WriteErrorPersistedKeypair { path, err })
}

pub(crate) async fn remove_keypair(
    keypairs_dir: &Path,
    worker_id: WorkerId,
) -> Result<(), KeyStorageError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    tokio::fs::remove_file(path.as_path())
        .await
        .map_err(|err| RemoveErrorPersistedKeypair {
            path,
            worker_id,
            err,
        })?;
    Ok(())
}

pub(crate) async fn persist_worker(
    workers_dir: &Path,
    worker_id: WorkerId,
    worker: PersistedWorker,
) -> Result<(), WorkersError> {
    let path = workers_dir.join(worker_file_name(worker_id));
    let bytes =
        toml::to_vec(&worker).map_err(|err| WorkersError::SerializePersistedWorker { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WorkersError::WriteErrorPersistedWorker { path, err })
}

pub(crate) async fn remove_worker(
    workers_dir: &Path,
    worker_id: WorkerId,
) -> Result<(), WorkersError> {
    let path = workers_dir.join(worker_file_name(worker_id));
    tokio::fs::remove_file(path.as_path()).await.map_err(|err| {
        WorkersError::RemoveErrorPersistedWorker {
            path,
            worker_id,
            err,
        }
    })
}

/// Load info about persisted workers from disk in parallel
pub(crate) async fn load_persisted_workers(
    workers_dir: &Path,
) -> eyre::Result<Vec<(PersistedWorker, PathBuf)>> {
    let workers = fs_utils::load_persisted_data(workers_dir, is_worker, |bytes| {
        toml::from_slice(bytes).map_err(|e| e.into())
    })
    .await?;

    Ok(workers)
}

/// Load info about persisted key pairs from disk in parallel
pub(crate) async fn load_persisted_key_pairs(
    key_pairs_dir: &Path,
) -> eyre::Result<Vec<(PersistedKeypair, PathBuf)>> {
    let key_pairs = fs_utils::load_persisted_data(key_pairs_dir, is_keypair, |bytes| {
        toml::from_slice(bytes).map_err(|e| e.into())
    })
    .await?;

    Ok(key_pairs)
}
