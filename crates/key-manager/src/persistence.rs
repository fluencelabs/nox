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

use crate::error::KeyManagerError::{
    CannotExtractRSASecretKey, CreateKeypairsDir, DeserializePersistedKeypair,
    ReadPersistedKeypair, SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use crate::error::{KeyManagerError, WorkerRegistryError};
use crate::worker_registry::WorkerInfo;
use crate::KeyManagerError::{
    PersistedKeypairDecodingError, PersistedKeypairInvalidKeyformat, RemoveErrorPersistedKeypair,
};
use fluence_keypair::{KeyFormat, KeyPair};
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

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
    type Error = KeyManagerError;

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
pub async fn persist_keypair(
    keypairs_dir: &Path,
    worker_id: PeerId,
    persisted_keypair: PersistedKeypair,
) -> Result<(), KeyManagerError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    let bytes =
        toml::to_vec(&persisted_keypair).map_err(|err| SerializePersistedKeypair { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WriteErrorPersistedKeypair { path, err })
}

async fn load_persisted_keypair(file: &Path) -> Result<KeyPair, KeyManagerError> {
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

pub(crate) async fn load_persisted_worker(
    file: &Path,
) -> Result<PersistedWorker, WorkerRegistryError> {
    let bytes =
        tokio::fs::read(file)
            .await
            .map_err(|err| WorkerRegistryError::ReadPersistedWorker {
                err,
                path: file.to_path_buf(),
            })?;

    toml::from_slice(bytes.as_slice()).map_err(|err| {
        WorkerRegistryError::DeserializePersistedWorker {
            err,
            path: file.to_path_buf(),
        }
    })
}

/// Load info about persisted keypairs from disk
pub(crate) async fn load_persisted_key_pairs(
    key_pairs_dir: &Path,
) -> Result<Vec<KeyPair>, KeyManagerError> {
    let list_files = tokio::fs::read_dir(key_pairs_dir).await.ok();

    let files =
        match list_files {
            Some(mut entries) => {
                let mut paths = vec![];
                while let Some(entry) = entries.next_entry().await.map_err(|err| {
                    KeyManagerError::DirectoryListError {
                        path: key_pairs_dir.to_path_buf(),
                        err,
                    }
                })? {
                    paths.push(entry.path())
                }
                paths
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

    let mut keypairs = vec![];
    for file in files.iter() {
        let res: eyre::Result<()> = try {
            if is_keypair(file) {
                keypairs.push(load_persisted_keypair(file).await?);
            }
        };

        if let Err(err) = res {
            log::warn!("{err}")
        }
    }

    Ok(keypairs)
}

pub async fn remove_keypair(keypairs_dir: &Path, worker_id: PeerId) -> Result<(), KeyManagerError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    tokio::fs::remove_file(path.clone())
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
) -> Result<(), WorkerRegistryError> {
    let path = workers_dir.join(worker_file_name(worker_id));
    let bytes = toml::to_vec(&worker)
        .map_err(|err| WorkerRegistryError::SerializePersistedWorker { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WorkerRegistryError::WriteErrorPersistedWorker { path, err })
}

pub async fn remove_worker(
    workers_dir: &Path,
    worker_id: PeerId,
) -> Result<(), WorkerRegistryError> {
    let path = workers_dir.join(worker_file_name(worker_id));
    tokio::fs::remove_file(path.clone()).await.map_err(|err| {
        WorkerRegistryError::RemoveErrorPersistedWorker {
            path,
            worker_id,
            err,
        }
    })
}
