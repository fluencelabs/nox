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

use crate::error::KeyManagerError;
use crate::persistence::{load_persisted_keypairs, persist_keypair, PersistedKeypair};
use crate::KeyManagerError::{WorkerAlreadyExists, WorkerNotFound, WorkerNotFoundByDeal};
use parking_lot::RwLock;

pub const INSECURE_KEYPAIR_SEED: Range<u8> = 0..32;

type DealId = String;

#[derive(Clone)]
pub struct KeyManager {
    /// worker_id -> worker_keypair
    worker_keypairs: Arc<RwLock<HashMap<PeerId, KeyPair>>>,
    /// deal_id -> worker_id
    worker_ids: Arc<RwLock<HashMap<DealId, PeerId>>>,
    /// worker_id -> init_peer_id of worker creator
    worker_creators: Arc<RwLock<HashMap<PeerId, PeerId>>>,
    keypairs_dir: PathBuf,
    host_peer_id: PeerId,
    // temporary public, will refactor
    pub insecure_keypair: KeyPair,
    management_peer_id: PeerId,
}

impl KeyManager {
    pub fn new(keypairs_dir: PathBuf, host_peer_id: PeerId, management_peer_id: PeerId) -> Self {
        let this = Self {
            worker_keypairs: Arc::new(Default::default()),
            worker_ids: Arc::new(Default::default()),
            worker_creators: Arc::new(Default::default()),
            keypairs_dir,
            host_peer_id,
            insecure_keypair: KeyPair::from_secret_key(
                INSECURE_KEYPAIR_SEED.collect(),
                KeyFormat::Ed25519,
            )
            .expect("error creating insecure keypair"),
            management_peer_id,
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
                self.worker_ids
                    .write()
                    .insert(persisted_kp.deal_id, keypair.get_peer_id());

                self.worker_keypairs.write().insert(peer_id, keypair);
            };

            if let Err(e) = res {
                log::warn!("Failed to restore persisted keypair: {}", e);
            }
        }
    }

    pub fn is_local(&self, peer_id: PeerId) -> bool {
        self.is_host(peer_id) || self.is_worker(peer_id)
    }

    pub fn is_host(&self, peer_id: PeerId) -> bool {
        self.host_peer_id == peer_id
    }

    pub fn is_worker(&self, peer_id: PeerId) -> bool {
        self.worker_keypairs.read().contains_key(&peer_id)
    }

    pub fn is_management(&self, peer_id: PeerId) -> bool {
        self.management_peer_id == peer_id
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }

    pub fn generate_deal_id(init_peer_id: PeerId) -> String {
        format!("direct_hosting_{init_peer_id}")
    }

    pub fn create_worker(
        &self,
        deal_id: Option<String>,
        init_peer_id: PeerId,
    ) -> Result<PeerId, KeyManagerError> {
        // if deal_id is not provided, we associate it with init_peer_id
        let deal_id = deal_id.unwrap_or(Self::generate_deal_id(init_peer_id));
        let worker_id = self.worker_ids.read().get(&deal_id).cloned();
        match worker_id {
            Some(_) => Err(WorkerAlreadyExists { deal_id }),
            _ => {
                let kp = self.generate_keypair();
                let worker_id = kp.get_peer_id();
                self.store_keypair(deal_id, init_peer_id, kp)?;
                Ok(worker_id)
            }
        }
    }

    pub fn get_worker_id(&self, deal_id: String) -> Result<PeerId, KeyManagerError> {
        self.worker_ids
            .read()
            .get(&deal_id)
            .cloned()
            .ok_or(WorkerNotFoundByDeal(deal_id))
    }

    pub fn get_worker_keypair(&self, worker_id: PeerId) -> Result<KeyPair, KeyManagerError> {
        self.worker_keypairs
            .read()
            .get(&worker_id)
            .cloned()
            .ok_or(KeyManagerError::KeypairNotFound(worker_id))
    }

    pub fn get_worker_creator(&self, worker_id: PeerId) -> Result<PeerId, KeyManagerError> {
        self.worker_creators
            .read()
            .get(&worker_id)
            .cloned()
            .ok_or(WorkerNotFound(worker_id))
    }

    pub fn generate_keypair(&self) -> KeyPair {
        KeyPair::generate_ed25519()
    }

    pub fn store_keypair(
        &self,
        deal_id: DealId,
        deal_creator: PeerId,
        keypair: KeyPair,
    ) -> Result<(), KeyManagerError> {
        persist_keypair(
            &self.keypairs_dir,
            PersistedKeypair::new(deal_creator, &keypair, deal_id.clone())?,
        )?;
        let worker_id = keypair.get_peer_id();
        self.worker_ids.write().insert(deal_id, worker_id);
        self.worker_creators.write().insert(worker_id, deal_creator);
        self.worker_keypairs.write().insert(worker_id, keypair);

        Ok(())
    }
}
