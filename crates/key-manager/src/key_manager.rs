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

use fluence_keypair::{KeyFormat, KeyPair};
use libp2p::PeerId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::persistence::{load_persisted_keypairs, persist_keypair, PersistedKeypair};
use parking_lot::RwLock;

#[derive(Clone)]
pub struct KeyManager {
    local_peer_ids: Arc<RwLock<HashMap<PeerId, Arc<KeyPair>>>>,
    remote_peer_ids: Arc<RwLock<HashMap<PeerId, Arc<KeyPair>>>>,
    keypairs_dir: PathBuf,
    host_peer_id: PeerId,
}

impl KeyManager {
    pub fn new(keypairs_dir: PathBuf, host_peer_id: PeerId) -> Self {
        let this = Self {
            local_peer_ids: Arc::new(Default::default()),
            remote_peer_ids: Arc::new(Default::default()),
            keypairs_dir,
            host_peer_id,
        };

        this.load_persisted_keypairs();
        this
    }

    pub fn load_persisted_keypairs(&self) {
        let persisted_keypairs = load_persisted_keypairs(&self.keypairs_dir);

        for pkp in persisted_keypairs {
            let res: eyre::Result<()> = try {
                let persisted_kp = pkp?;
                let keypair = Arc::new(KeyPair::from_vec(
                    persisted_kp.keypair_bytes,
                    KeyFormat::from_str(&persisted_kp.key_format)?,
                )?);
                let peer_id = keypair.get_peer_id().to_base58();
                self.remote_peer_ids
                    .write()
                    .insert(persisted_kp.remote_peer_id, keypair.clone());

                self.local_peer_ids.write().insert(peer_id, keypair.clone());

                ()
            };

            if let Err(e) = res {
                log::warn!("Failed to restore persisted keypair: {}", e);
            }
        }
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }

    pub fn has_keypair(&self, remote_peer_id: &str) -> bool {
        self.remote_peer_ids.read().contains_key(remote_peer_id)
    }

    pub fn is_local_peer_id(&self, local_peer_id: PeerId) -> bool {
        self.local_peer_ids.read().contains_key(PeerId)
    }

    pub fn get_keypair_by_remote_peer_id(
        &self,
        remote_peer_id: &str,
    ) -> eyre::Result<Arc<KeyPair>> {
        if let Some(k) = self.remote_peer_ids.read().get(remote_peer_id).cloned() {
            Ok(k)
        } else {
            Err(eyre::eyre!(
                "Keypair for peer id {} not exists",
                remote_peer_id
            ))
        }
    }

    pub fn get_keypair_by_local_peer_id(&self, local_peer_id: &str) -> eyre::Result<Arc<KeyPair>> {
        if let Some(k) = self.local_peer_ids.read().get(local_peer_id).cloned() {
            Ok(k)
        } else {
            Err(eyre::eyre!(
                "Keypair for peer id {} not exists",
                local_peer_id
            ))
        }
    }

    pub fn generate_keypair(&self) -> Arc<KeyPair> {
        Arc::new(KeyPair::generate_ed25519())
    }

    pub fn store_keypair(&self, remote_peer_id: &str, keypair: Arc<KeyPair>) -> eyre::Result<()> {
        persist_keypair(
            &self.keypairs_dir,
            PersistedKeypair::new(remote_peer_id.to_string(), &keypair),
        )?;
        let peer_id = keypair.get_peer_id().to_base58();
        self.remote_peer_ids
            .write()
            .insert(remote_peer_id.to_string(), keypair.clone());

        self.local_peer_ids.write().insert(peer_id, keypair.clone());

        Ok(())
    }
}
