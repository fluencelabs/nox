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
use std::ops::Range;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::error::{KeyManagerError, PersistedKeypairError};
use crate::persistence::{load_persisted_keypairs, persist_keypair, PersistedKeypair};
use parking_lot::RwLock;

pub const INSECURE_KEYPAIR_SEED: Range<u8> = 0..32;

#[derive(Clone)]
pub struct KeyManager {
    /// scope_peer_id -> scope_keypair
    scope_keypairs: Arc<RwLock<HashMap<PeerId, KeyPair>>>,
    /// remote_peer_id -> scope_peer_id
    scope_peer_ids: Arc<RwLock<HashMap<PeerId, PeerId>>>,
    keypairs_dir: PathBuf,
    host_peer_id: PeerId,
    // temporary public, will refactor
    pub insecure_keypair: KeyPair,
}

impl KeyManager {
    pub fn new(keypairs_dir: PathBuf, host_peer_id: PeerId) -> Self {
        let this = Self {
            scope_keypairs: Arc::new(Default::default()),
            scope_peer_ids: Arc::new(Default::default()),
            keypairs_dir,
            host_peer_id,
            insecure_keypair: KeyPair::from_secret_key(
                INSECURE_KEYPAIR_SEED.collect(),
                KeyFormat::Ed25519,
            )
            .expect("error creating insecure keypair"),
        };

        this.load_persisted_keypairs();
        this
    }

    pub fn load_persisted_keypairs(&self) {
        let persisted_keypairs = load_persisted_keypairs(&self.keypairs_dir);

        for pkp in persisted_keypairs {
            let res: eyre::Result<()> = try {
                let persisted_kp = pkp?;
                let keypair = KeyPair::from_secret_key(
                    persisted_kp.private_key_bytes,
                    KeyFormat::from_str(&persisted_kp.key_format)?,
                )?;
                let peer_id = keypair.get_peer_id();
                self.scope_peer_ids
                    .write()
                    .insert(persisted_kp.remote_peer_id, keypair.get_peer_id());

                self.scope_keypairs.write().insert(peer_id, keypair);
            };

            if let Err(e) = res {
                log::warn!("Failed to restore persisted keypair: {}", e);
            }
        }
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }

    pub fn has_keypair(&self, remote_peer_id: PeerId) -> bool {
        self.scope_peer_ids.read().contains_key(&remote_peer_id)
    }

    pub fn is_scope_peer_id(&self, scope_peer_id: PeerId) -> bool {
        self.scope_keypairs.read().contains_key(&scope_peer_id)
    }

    /// For local peer ids is identity,
    /// for remote returns associated peer id or generate a new one.
    pub fn get_scope_peer_id(&self, init_peer_id: PeerId) -> Result<PeerId, PersistedKeypairError> {
        // All "nested" spells share the same keypair.
        // "nested" means spells which are created by other spells
        if self.is_scope_peer_id(init_peer_id) {
            Ok(init_peer_id)
        } else {
            let scope_peer_id = self.scope_peer_ids.read().get(&init_peer_id).cloned();
            match scope_peer_id {
                Some(p) => Ok(p),
                _ => {
                    let kp = self.generate_keypair();
                    let scope_peer_id = kp.get_peer_id();
                    self.store_keypair(init_peer_id, kp)?;
                    Ok(scope_peer_id)
                }
            }
        }
    }

    pub fn get_scope_keypair(&self, scope_peer_id: PeerId) -> Result<KeyPair, KeyManagerError> {
        self.scope_keypairs
            .read()
            .get(&scope_peer_id)
            .cloned()
            .ok_or(KeyManagerError::KeypairNotFound(scope_peer_id))
    }

    pub fn generate_keypair(&self) -> KeyPair {
        KeyPair::generate_ed25519()
    }

    pub fn store_keypair(
        &self,
        remote_peer_id: PeerId,
        keypair: KeyPair,
    ) -> Result<(), PersistedKeypairError> {
        persist_keypair(
            &self.keypairs_dir,
            PersistedKeypair::new(remote_peer_id, &keypair)?,
        )?;
        let scope_peer_id = keypair.get_peer_id();
        self.scope_peer_ids
            .write()
            .insert(remote_peer_id, scope_peer_id);

        self.scope_keypairs.write().insert(scope_peer_id, keypair);

        Ok(())
    }
}
