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

#[cfg(test)]
mod tests {
    use crate::KeyStorage;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_key_storage_creation() {
        // Create a temporary directory for key storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().to_path_buf();

        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a KeyStorage instance from a path
        let loaded_key_storage = KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
            .await
            .expect("Failed to create KeyStorage from path");

        // Check that the loaded key storage has the correct initial state
        assert_eq!(loaded_key_storage.worker_key_pairs.read().len(), 0);
        assert_eq!(loaded_key_storage.key_pairs_dir, key_pairs_dir);
        assert_eq!(
            loaded_key_storage.root_key_pair.to_vec(),
            root_key_pair.to_vec()
        );
    }

    #[tokio::test]
    async fn test_key_pair_creation_and_removal() {
        // Create a temporary directory for key storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().to_path_buf();

        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a KeyStorage instance from a path
        let key_storage = KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
            .await
            .expect("Failed to create KeyStorage from path");

        // Create a key pair and check that it is added to the storage
        let key_pair_1 = key_storage
            .create_key_pair()
            .await
            .expect("Failed to create key pair 1");
        assert_eq!(
            key_storage
                .get_key_pair(key_pair_1.get_peer_id())
                .map(|k| k.to_vec()),
            Some(key_pair_1.to_vec())
        );

        // Create another key pair and check that it is added to the storage
        let key_pair_2 = key_storage
            .create_key_pair()
            .await
            .expect("Failed to create key pair 2");
        assert_eq!(
            key_storage
                .get_key_pair(key_pair_2.get_peer_id())
                .map(|k| k.to_vec()),
            Some(key_pair_2.to_vec())
        );

        // Remove the first key pair and check that it is removed from the storage
        key_storage
            .remove_key_pair(key_pair_1.get_peer_id())
            .await
            .expect("Failed to remove key pair 1");
        assert_eq!(
            key_storage
                .get_key_pair(key_pair_1.get_peer_id())
                .map(|k| k.to_vec()),
            None
        );

        // Remove the second key pair and check that it is removed from the storage
        key_storage
            .remove_key_pair(key_pair_2.get_peer_id())
            .await
            .expect("Failed to remove key pair 2");
        assert_eq!(
            key_storage
                .get_key_pair(key_pair_2.get_peer_id())
                .map(|k| k.to_vec()),
            None
        );
    }

    #[tokio::test]
    async fn test_persistence() {
        // Create a temporary directory for key storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().to_path_buf();

        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a KeyStorage instance from a path
        let key_storage_1 = KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
            .await
            .expect("Failed to create KeyStorage from path");

        // Create a key pair and check that it is added to the storage
        let key_pair_1 = key_storage_1
            .create_key_pair()
            .await
            .expect("Failed to create key pair 1");
        assert_eq!(
            key_storage_1
                .get_key_pair(key_pair_1.get_peer_id())
                .map(|k| k.to_vec()),
            Some(key_pair_1.to_vec())
        );
        drop(key_storage_1);

        let key_storage_2 = KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
            .await
            .expect("Failed to create KeyStorage from path");

        assert_eq!(
            key_storage_2
                .get_key_pair(key_pair_1.get_peer_id())
                .map(|k| k.to_vec()),
            Some(key_pair_1.to_vec())
        );
    }
}
