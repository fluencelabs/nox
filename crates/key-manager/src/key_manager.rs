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
use crate::persistence::{
    load_persisted_keypairs, persist_keypair, remove_keypair, PersistedKeypair,
};
use crate::KeyManagerError::{WorkerAlreadyExists, WorkerNotFound, WorkerNotFoundByDeal};
use parking_lot::RwLock;

pub const INSECURE_KEYPAIR_SEED: Range<u8> = 0..32;

type DealId = String;
type WorkerId = PeerId;

#[derive(Clone)]
pub struct WorkerInfo {
    pub deal_id: String,
    pub creator: PeerId,
}

#[derive(Clone)]
pub struct KeyManager {
    /// worker_id -> worker_keypair
    worker_keypairs: Arc<RwLock<HashMap<WorkerId, KeyPair>>>,
    worker_ids: Arc<RwLock<HashMap<DealId, WorkerId>>>,
    worker_infos: Arc<RwLock<HashMap<WorkerId, WorkerInfo>>>,
    keypairs_dir: PathBuf,
    host_peer_id: PeerId,
    // temporary public, will refactor
    pub insecure_keypair: KeyPair,
    pub root_keypair: KeyPair,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
}

impl KeyManager {
    pub fn new(
        keypairs_dir: PathBuf,
        root_keypair: KeyPair,
        management_peer_id: PeerId,
        builtins_management_peer_id: PeerId,
    ) -> Self {
        let this = Self {
            worker_keypairs: Arc::new(Default::default()),
            worker_ids: Arc::new(Default::default()),
            worker_infos: Arc::new(Default::default()),
            keypairs_dir,
            host_peer_id: root_keypair.get_peer_id(),
            insecure_keypair: KeyPair::from_secret_key(
                INSECURE_KEYPAIR_SEED.collect(),
                KeyFormat::Ed25519,
            )
            .expect("error creating insecure keypair"),
            root_keypair,
            management_peer_id,
            builtins_management_peer_id,
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
                let worker_id = keypair.get_peer_id();
                self.worker_ids
                    .write()
                    .insert(persisted_kp.deal_id.clone(), keypair.get_peer_id());

                self.worker_keypairs.write().insert(worker_id, keypair);

                self.worker_infos.write().insert(
                    worker_id,
                    WorkerInfo {
                        deal_id: persisted_kp.deal_id,
                        creator: persisted_kp.deal_creator,
                    },
                );
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
        self.management_peer_id == peer_id || self.builtins_management_peer_id == peer_id
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

    pub fn get_worker_id(
        &self,
        deal_id: Option<String>,
        init_peer_id: PeerId,
    ) -> Result<PeerId, KeyManagerError> {
        // if deal_id is not provided, we associate it with init_peer_id
        let deal_id = deal_id.unwrap_or(Self::generate_deal_id(init_peer_id));
        self.worker_ids
            .read()
            .get(&deal_id)
            .cloned()
            .ok_or(WorkerNotFoundByDeal(deal_id))
    }

    pub fn list_workers(&self) -> Vec<WorkerId> {
        self.worker_infos.read().keys().cloned().collect()
    }

    pub fn get_deal_id(&self, worker_id: PeerId) -> Result<DealId, KeyManagerError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .ok_or_else(|| KeyManagerError::WorkerNotFound(worker_id))
            .map(|info| info.deal_id.clone())
    }

    pub fn remove_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        let deal_id = self.get_deal_id(worker_id)?;
        remove_keypair(&self.keypairs_dir, &deal_id)?;
        let removed_worker_id = self.worker_ids.write().remove(&deal_id);
        let removed_worker_info = self.worker_infos.write().remove(&worker_id);
        let removed_worker_kp = self.worker_keypairs.write().remove(&worker_id);

        debug_assert!(removed_worker_id.is_some(), "worker_id does not exist");
        debug_assert!(removed_worker_info.is_some(), "worker info does not exist");
        debug_assert!(removed_worker_kp.is_some(), "worker kp does not exist");

        Ok(())
    }

    pub fn get_worker_keypair(&self, worker_id: PeerId) -> Result<KeyPair, KeyManagerError> {
        if self.is_host(worker_id) {
            Ok(self.root_keypair.clone())
        } else {
            self.worker_keypairs
                .read()
                .get(&worker_id)
                .cloned()
                .ok_or(KeyManagerError::KeypairNotFound(worker_id))
        }
    }

    pub fn get_worker_creator(&self, worker_id: PeerId) -> Result<PeerId, KeyManagerError> {
        if self.is_host(worker_id) {
            Ok(worker_id)
        } else {
            self.worker_infos
                .read()
                .get(&worker_id)
                .cloned()
                .ok_or(WorkerNotFound(worker_id))
                .map(|i| i.creator)
        }
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
        self.worker_ids.write().insert(deal_id.clone(), worker_id);
        self.worker_infos.write().insert(
            worker_id,
            WorkerInfo {
                deal_id,
                creator: deal_creator,
            },
        );
        self.worker_keypairs.write().insert(worker_id, keypair);

        Ok(())
    }
}
