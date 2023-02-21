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

use fs_utils::{create_dir, list_files};

use crate::error::KeyManagerError;
use crate::error::KeyManagerError::{
    CannotExtractRSASecretKey, CreateKeypairsDir, DeserializePersistedKeypair,
    ReadPersistedKeypair, SerializePersistedKeypair, WriteErrorPersistedKeypair,
};
use fluence_keypair::KeyPair;
use fluence_libp2p::peerid_serializer;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistedKeypair {
    #[serde(with = "peerid_serializer")]
    pub deal_creator: PeerId,
    pub private_key_bytes: Vec<u8>,
    pub key_format: String,
    pub deal_id: String,
}

impl PersistedKeypair {
    pub fn new(
        deal_creator: PeerId,
        keypair: &KeyPair,
        deal_id: String,
    ) -> Result<Self, KeyManagerError> {
        Ok(Self {
            deal_creator,
            private_key_bytes: keypair.secret().map_err(|_| CannotExtractRSASecretKey)?,
            key_format: keypair.public().get_key_format().into(),
            deal_id,
        })
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
) -> Result<(), KeyManagerError> {
    let path = keypairs_dir.join(keypair_file_name(&persisted_keypair.deal_id));
    let bytes =
        toml::to_vec(&persisted_keypair).map_err(|err| SerializePersistedKeypair { err })?;
    std::fs::write(&path, bytes).map_err(|err| WriteErrorPersistedKeypair { path, err })
}

/// Load info about persisted keypairs from disk
pub fn load_persisted_keypairs(
    keypairs_dir: &Path,
) -> Vec<Result<PersistedKeypair, KeyManagerError>> {
    // Load all persisted service file names
    let files = match list_files(keypairs_dir) {
        Some(files) => files,
        None => {
            // Attempt to create directory and exit
            if let Err(err) = create_dir(keypairs_dir) {
                return vec![Err(CreateKeypairsDir {
                    path: keypairs_dir.to_path_buf(),
                    err,
                })];
            }

            return vec![];
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
