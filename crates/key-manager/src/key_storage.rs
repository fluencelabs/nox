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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use libp2p::PeerId;
use parking_lot::RwLock;

use crate::persistence::{load_persisted_key_pairs, persist_keypair, remove_keypair};
use crate::{KeyManagerError, WorkerId};
use fluence_keypair::KeyPair;

pub struct KeyStorage {
    /// worker_id -> worker_keypair
    worker_key_pairs: RwLock<HashMap<WorkerId, KeyPair>>,
    key_pairs_dir: PathBuf,
    pub root_key_pair: KeyPair,
}

impl KeyStorage {
    pub fn new(root_key_pair: KeyPair) -> Self {
        Self {
            worker_key_pairs: Default::default(),
            key_pairs_dir: Default::default(),
            root_key_pair,
        }
    }

    pub async fn from_path(
        key_pairs_dir: &Path,
        root_key_pair: KeyPair,
    ) -> Result<Self, KeyManagerError> {
        let key_pairs = load_persisted_key_pairs(key_pairs_dir).await?;

        let mut worker_key_pairs = HashMap::with_capacity(key_pairs.len());
        for keypair in key_pairs {
            let worker_id = keypair.get_peer_id();
            worker_key_pairs.insert(worker_id, keypair);
        }
        Ok(Self {
            worker_key_pairs: RwLock::new(worker_key_pairs),
            key_pairs_dir: key_pairs_dir.to_path_buf(),
            root_key_pair,
        })
    }

    pub fn get_key_pair(&self, worker_id: PeerId) -> Option<KeyPair> {
        self.worker_key_pairs.read().get(&worker_id).cloned()
    }

    pub async fn create_key_pair(&self) -> Result<KeyPair, KeyManagerError> {
        let keypair = KeyPair::generate_ed25519();
        let worker_id = keypair.get_peer_id();
        persist_keypair(&self.key_pairs_dir, worker_id, (&keypair).try_into()?).await?;
        let mut guard = self.worker_key_pairs.write();
        guard.insert(worker_id, keypair.clone());
        Ok(keypair)
    }

    pub async fn remove_key_pair(&self, worker_id: WorkerId) -> Result<(), KeyManagerError> {
        remove_keypair(&self.key_pairs_dir, worker_id).await?;
        let mut guard = self.worker_key_pairs.write();
        guard.remove(&worker_id);
        Ok(())
    }
}
