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
    CannotExtractRSASecretKey, CreateKeypairsDir, DeserializePersistedKeypair,
    ReadPersistedKeypair, SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use crate::error::{KeyStorageError, WorkersError};
use crate::workers::WorkerInfo;
use crate::KeyStorageError::{
    PersistedKeypairDecodingError, PersistedKeypairInvalidKeyformat, RemoveErrorPersistedKeypair,
};
use crate::DEFAULT_PARALLELISM;
use fluence_keypair::{KeyFormat, KeyPair};
use fluence_libp2p::peerid_serializer;
use libp2p::futures::StreamExt;
use libp2p::PeerId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::str::FromStr;
use std::thread::available_parallelism;
use tokio::fs::DirEntry;
use tokio_stream::wrappers::ReadDirStream;

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
    #[serde(with = "peerid_serializer")]
    pub worker_id: PeerId,
    #[serde(with = "peerid_serializer")]
    pub creator: PeerId,
    #[serde(default)]
    pub deal_id: String,
    #[serde(default = "default_bool::<true>")]
    pub active: bool,
}

impl From<PersistedWorker> for WorkerInfo {
    fn from(val: PersistedWorker) -> Self {
        WorkerInfo {
            deal_id: val.deal_id,
            creator: val.creator,
            active: RwLock::new(val.active),
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

pub fn keypair_file_name(worker_id: PeerId) -> String {
    format!("{}_keypair.toml", worker_id.to_base58())
}

pub(crate) fn worker_file_name(worker_id: PeerId) -> String {
    format!("{}_info.toml", worker_id.to_base58())
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
    worker_id: PeerId,
    persisted_keypair: PersistedKeypair,
) -> Result<(), KeyStorageError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    let bytes =
        toml::to_vec(&persisted_keypair).map_err(|err| SerializePersistedKeypair { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WriteErrorPersistedKeypair { path, err })
}

async fn load_persisted_keypair(file: &Path) -> Result<KeyPair, KeyStorageError> {
    let bytes = tokio::fs::read(file)
        .await
        .map_err(|err| ReadPersistedKeypair {
            err,
            path: file.to_path_buf(),
        })?;
    let keypair: PersistedKeypair =
        toml::from_slice(bytes.as_slice()).map_err(|err| DeserializePersistedKeypair {
            err,
            path: file.to_path_buf(),
        })?;

    KeyPair::from_secret_key(
        keypair.private_key_bytes,
        KeyFormat::from_str(&keypair.key_format).map_err(|err| {
            PersistedKeypairInvalidKeyformat {
                err,
                path: file.to_path_buf(),
            }
        })?,
    )
    .map_err(|err| PersistedKeypairDecodingError {
        err,
        path: file.to_path_buf(),
    })
}

pub(crate) async fn load_persisted_worker(file: &Path) -> Result<PersistedWorker, WorkersError> {
    let bytes = tokio::fs::read(file)
        .await
        .map_err(|err| WorkersError::ReadPersistedWorker {
            err,
            path: file.to_path_buf(),
        })?;

    toml::from_slice(bytes.as_slice()).map_err(|err| WorkersError::DeserializePersistedWorker {
        err,
        path: file.to_path_buf(),
    })
}

fn process_key_pair_dir_entry(
    entry: DirEntry,
) -> Option<impl Future<Output = Option<KeyPair>> + Sized> {
    let path = entry.path();
    if is_keypair(path.as_path()) {
        let task = async move {
            let res: eyre::Result<KeyPair> = try { load_persisted_keypair(path.as_path()).await? };
            if let Err(err) = &res {
                log::warn!("Failed to load key pair: {err}")
            }
            res.ok()
        };

        Some(task)
    } else {
        None
    }
}

/// Load info about persisted key pairs from disk
pub(crate) async fn load_persisted_key_pairs(
    key_pairs_dir: &Path,
) -> Result<Vec<KeyPair>, KeyStorageError> {
    let parallelism = available_parallelism()
        .map(|x| x.get())
        .unwrap_or(DEFAULT_PARALLELISM);
    let list_files = tokio::fs::read_dir(key_pairs_dir).await.ok();

    let keypairs = match list_files {
        Some(entries) => {
            let keypairs: Vec<KeyPair> = ReadDirStream::new(entries)
                .filter_map(|res| async {
                    match res {
                        Ok(entry) => process_key_pair_dir_entry(entry),
                        Err(err) => {
                            log::warn!("Could not read key pairs directory: {err}");
                            None
                        }
                    }
                })
                .buffer_unordered(parallelism)
                .filter_map(|e| async { e })
                .collect()
                .await;

            keypairs
        }
        None => {
            // Attempt to create directory
            tokio::fs::create_dir_all(key_pairs_dir)
                .await
                .map_err(|err| CreateKeypairsDir {
                    path: key_pairs_dir.to_path_buf(),
                    err,
                })?;
            vec![]
        }
    };

    Ok(keypairs)
}

pub(crate) async fn remove_keypair(
    keypairs_dir: &Path,
    worker_id: PeerId,
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
    worker_id: PeerId,
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
    worker_id: PeerId,
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

/// Load info about persisted workers from disk
pub(crate) async fn load_persisted_workers(
    workers_dir: &Path,
) -> eyre::Result<Vec<PersistedWorker>> {
    let parallelism = available_parallelism()
        .map(|x| x.get())
        .unwrap_or(DEFAULT_PARALLELISM);
    let list_files = tokio::fs::read_dir(workers_dir).await.ok();
    let workers = match list_files {
        Some(entries) => {
            let workers: Vec<PersistedWorker> = ReadDirStream::new(entries)
                .filter_map(|res| async {
                    match res {
                        Ok(entry) => process_worker_dir_entry(entry),
                        Err(err) => {
                            log::warn!("Could not read workers directory: {err}");
                            None
                        }
                    }
                })
                .buffer_unordered(parallelism)
                .filter_map(|e| async { e })
                .collect()
                .await;

            workers
        }
        None => {
            // Attempt to create directory
            tokio::fs::create_dir_all(workers_dir)
                .await
                .map_err(|err| WorkersError::CreateWorkersDir {
                    path: workers_dir.to_path_buf(),
                    err,
                })?;
            vec![]
        }
    };

    Ok(workers)
}

fn process_worker_dir_entry(
    entry: DirEntry,
) -> Option<impl Future<Output = Option<PersistedWorker>> + Sized> {
    let path = entry.path();
    if is_worker(path.as_path()) {
        let task = async move {
            let res: eyre::Result<PersistedWorker> =
                try { load_persisted_worker(path.as_path()).await? };
            if let Err(err) = &res {
                log::warn!("Failed to load worker: {err}")
            }
            res.ok()
        };

        Some(task)
    } else {
        None
    }
}
