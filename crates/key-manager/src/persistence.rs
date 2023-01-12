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

use fs_utils::{create_dirs, list_files};

use crate::error::PersistedKeypairError;
use crate::error::PersistedKeypairError::{
    CreateKeypairsDir, DeserializePersistedKeypair, ReadPersistedKeypair,
    SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use fluence_keypair::KeyPair;
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedKeypair {
    #[serde(with = "peerid_serializer")]
    pub remote_peer_id: PeerId,
    pub private_key_bytes: Vec<u8>,
    pub key_format: String,
}

impl PersistedKeypair {
    pub fn new(owner_id: PeerId, keypair: &KeyPair) -> Self {
        Self {
            remote_peer_id: owner_id,
            private_key_bytes: keypair.to_vec(),
            key_format: keypair.public().get_key_format().into(),
        }
    }
}

pub fn keypair_file_name(remote_peer_id: &str) -> String {
    format!("{remote_peer_id}_keypair.toml")
}

pub fn is_keypair(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n.ends_with("_keypair.toml"))
}

/// Persist keypair info to disk, so it is recreated after restart
pub fn persist_keypair(
    keypairs_dir: &Path,
    persisted_keypair: PersistedKeypair,
) -> Result<(), PersistedKeypairError> {
    let path = keypairs_dir.join(keypair_file_name(
        &persisted_keypair.remote_peer_id.to_base58(),
    ));
    let bytes =
        toml::to_vec(&persisted_keypair).map_err(|err| SerializePersistedKeypair { err })?;
    std::fs::write(&path, bytes).map_err(|err| WriteErrorPersistedKeypair { path, err })
}

/// Load info about persisted keypairs from disk
pub fn load_persisted_keypairs(
    keypairs_dir: &Path,
) -> Vec<Result<PersistedKeypair, PersistedKeypairError>> {
    // Load all persisted service file names
    let files = match list_files(keypairs_dir) {
        Some(files) => files,
        None => {
            // Attempt to create directory and exit
            return create_dirs(&[&keypairs_dir])
                .map_err(|err| CreateKeypairsDir {
                    path: keypairs_dir.to_path_buf(),
                    err,
                })
                .err()
                .into_iter()
                .map(Err)
                .collect();
        }
    };

    files
        .filter(|p| is_keypair(p))
        .map(|file| {
            // Load persisted keypair
            let bytes = std::fs::read(&file).map_err(|err| ReadPersistedKeypair {
                err,
                path: file.to_path_buf(),
            })?;
            let keypair =
                toml::from_slice(bytes.as_slice()).map_err(|err| DeserializePersistedKeypair {
                    err,
                    path: file.to_path_buf(),
                })?;

            Ok(keypair)
        })
        .collect()
}
