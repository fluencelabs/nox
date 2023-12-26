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
use std::path::Path;
use std::sync::Arc;

use libp2p::PeerId;
use parking_lot::RwLock;

use crate::persistence::load_persisted_key_pairs;
use crate::WorkerId;
use fluence_keypair::KeyPair;

#[derive(Clone)]
pub struct KeyStorage {
    /// worker_id -> worker_keypair
    worker_key_pairs: Arc<RwLock<HashMap<WorkerId, KeyPair>>>,
}

impl KeyStorage {
    pub fn new() -> Self {
        Self {
            worker_key_pairs: Arc::new(Default::default()),
        }
    }

    pub fn from_path(key_pairs_dir: &Path) -> eyre::Result<Self> {
        let key_pairs = load_persisted_key_pairs(key_pairs_dir)?;

        let mut worker_key_pairs = HashMap::with_capacity(key_pairs.len());
        for keypair in key_pairs {
            let worker_id = keypair.get_peer_id();
            worker_key_pairs.insert(worker_id, keypair);
        }
        Ok(Self {
            worker_key_pairs: Arc::new(RwLock::new(worker_key_pairs)),
        })
    }

    pub fn get_keypair(&self, worker_id: PeerId) -> Option<KeyPair> {
        self.worker_key_pairs.read().get(&worker_id).cloned()
    }
}
