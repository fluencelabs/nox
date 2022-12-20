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

use fluence_keypair::KeyPair;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::persistence::{load_persisted_keypairs, persist_keypair, PersistedKeypair};
use parking_lot::RwLock;

#[derive(Clone)]
pub struct KeyManager {
    local_peer_ids: Arc<RwLock<HashSet<String>>>,
    keypairs: Arc<RwLock<HashMap<String, KeyPair>>>,
    keypairs_dir: PathBuf,
}

impl KeyManager {
    pub fn new(keypairs_dir: PathBuf) -> Self {
        let this = Self {
            local_peer_ids: Arc::new(Default::default()),
            keypairs: Arc::new(Default::default()),
            keypairs_dir,
        };

        this.load_persisted_keypairs();
        this
    }

    pub fn load_persisted_keypairs(&self) {
        let persisted_keypairs = load_persisted_keypairs(&self.keypairs_dir);
    }

    pub fn get_or_generate_keypair(&self, owner_peer_id: &str) -> eyre::Result<KeyPair> {
        if let Some(k) = self.keypairs.read().get(owner_peer_id) {
            Ok(k.clone())
        } else {
            let new_kp = KeyPair::generate_ed25519();
            self.add_keypair(owner_peer_id, new_kp.clone())?;
            Ok(new_kp)
        }
    }

    pub fn add_keypair(&self, owner_peer_id: &str, keypair: KeyPair) -> eyre::Result<()> {
        persist_keypair(
            &self.keypairs_dir,
            PersistedKeypair::new(owner_peer_id.to_string(), &keypair),
        )?;
        self.keypairs
            .write()
            .insert(owner_peer_id.to_string(), keypair);

        Ok(())
    }

    pub fn has_local_peer_id(&self, local_peer_id: &str) -> bool {
        self.local_peer_ids.read().contains(local_peer_id)
    }
}
