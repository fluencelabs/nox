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
use std::path::{Path, PathBuf};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use core_distributor::CUID;
use fluence_keypair::KeyPair;
use libp2p::futures::StreamExt;
use tokio_stream::wrappers::ReadDirStream;
use types::peer_id;
use types::peer_scope::WorkerId;

use crate::error::KeyStorageError::{
    CannotExtractRSASecretKey, SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use crate::error::{KeyStorageError, WorkersError};
use crate::workers::WorkerInfo;
use crate::KeyStorageError::RemoveErrorPersistedKeypair;

pub const fn default_bool<const V: bool>() -> bool {
    V
}

pub const WORKER_INFO_FILE_NAME: &str = "info.toml";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedKeypair {
    pub private_key_bytes: Vec<u8>,
    pub key_format: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedWorker {
    pub worker_id: WorkerId,
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    pub creator: PeerId,
    #[serde(default)]
    pub deal_id: String,
    #[serde(default = "default_bool::<true>")]
    pub active: bool,
    pub cu_ids: Vec<CUID>,
    pub vm_flag: bool,
}

impl From<PersistedWorker> for WorkerInfo {
    fn from(val: PersistedWorker) -> Self {
        WorkerInfo {
            deal_id: val.deal_id.into(),
            creator: val.creator,
            active: val.active.into(),
            cu_ids: val.cu_ids,
            vm_flag: val.vm_flag.into(),
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

fn is_keypair(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_keypair.toml"))
}

/// Persist keypair info to disk, so it is recreated after restart
pub(crate) async fn persist_keypair(
    keypairs_dir: &Path,
    worker_id: WorkerId,
    persisted_keypair: PersistedKeypair,
) -> Result<(), KeyStorageError> {
    let path = keypairs_dir.join(keypair_file_name(worker_id));
    let bytes = toml_edit::ser::to_vec(&persisted_keypair)
        .map_err(|err| SerializePersistedKeypair { err })?;
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
    worker_dir: &Path,
    worker: PersistedWorker,
) -> Result<(), WorkersError> {
    let path = worker_dir.join(WORKER_INFO_FILE_NAME);
    let bytes = toml_edit::ser::to_vec(&worker)
        .map_err(|err| WorkersError::SerializePersistedWorker { err })?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|err| WorkersError::WriteErrorPersistedWorker { path, err })
}

pub(crate) async fn remove_worker(
    workers_dir: &Path,
    worker_id: WorkerId,
) -> Result<(), WorkersError> {
    let path = workers_dir.join(worker_id.to_string());
    tokio::fs::remove_dir_all(&path).await.map_err(|err| {
        WorkersError::RemoveErrorPersistedWorker {
            path,
            worker_id,
            err,
        }
    })?;
    Ok(())
}

/// Load info about persisted workers from disk in parallel
pub(crate) async fn load_persisted_workers(
    workers_dir: &Path,
) -> eyre::Result<Vec<PersistedWorker>> {
    let directory_list = tokio::fs::read_dir(workers_dir).await?;
    let directory_stream = ReadDirStream::new(directory_list);

    let workers = directory_stream
        .filter_map(|res| async move {
            match res {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_dir() {
                        let info_path = path.join(WORKER_INFO_FILE_NAME);
                        if info_path.is_file() {
                            Some(read_worker_info(info_path.clone()))
                        } else {
                            tracing::warn!("File {} isn't file. Skipping...", info_path.display());
                            None
                        }
                    } else {
                        tracing::debug!("File {} isn't directory. Skipping...", path.display());
                        None
                    }
                }
                Err(err) => {
                    tracing::warn!("Could not read worker directory: {err}");
                    None
                }
            }
        })
        .buffer_unordered(4)
        .filter_map(|e| async { e })
        //collect only loaded data and unwrap Option
        .collect()
        .await;

    Ok(workers)
}

async fn read_worker_info(info_file: PathBuf) -> Option<PersistedWorker> {
    let bytes = tokio::fs::read(&info_file).await;
    match bytes {
        Ok(bytes) => {
            let result = toml_edit::de::from_slice::<PersistedWorker>(&bytes);
            match result {
                Ok(worker) => Some(worker),
                Err(err) => {
                    tracing::warn!(
                        "Could not parse worker file {}: {}",
                        info_file.display(),
                        err
                    );
                    None
                }
            }
        }
        Err(err) => {
            tracing::warn!("Could not read file {}: {}", info_file.display(), err);
            None
        }
    }
}

/// Load info about persisted key pairs from disk in parallel
pub(crate) async fn load_persisted_key_pairs(
    key_pairs_dir: &Path,
) -> eyre::Result<Vec<(PersistedKeypair, PathBuf)>> {
    let key_pairs = fs_utils::load_persisted_data(key_pairs_dir, is_keypair, |bytes| {
        toml_edit::de::from_slice(bytes).map_err(|e| e.into())
    })
    .await?;

    Ok(key_pairs)
}
