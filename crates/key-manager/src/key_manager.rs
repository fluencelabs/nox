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
use libp2p::PeerId;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::KeyManagerError;
use crate::persistence::{
    load_persisted_keypairs_and_workers, persist_keypair, persist_worker, persist_worker_sync,
    remove_keypair, remove_worker, PersistedWorker,
};
use crate::KeyManagerError::{WorkerAlreadyExists, WorkerNotFound, WorkerNotFoundByDeal};
use parking_lot::RwLock;

type DealId = String;
type WorkerId = PeerId;

pub struct WorkerInfo {
    pub deal_id: String,
    pub creator: PeerId,
    pub active: RwLock<bool>,
}

#[derive(Clone)]
pub struct KeyManager {
    /// worker_id -> worker_keypair
    worker_keypairs: Arc<RwLock<HashMap<WorkerId, KeyPair>>>,
    worker_ids: Arc<RwLock<HashMap<DealId, WorkerId>>>,
    worker_infos: Arc<RwLock<HashMap<WorkerId, Arc<WorkerInfo>>>>,
    keypairs_dir: PathBuf,
    host_peer_id: PeerId,
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
            root_keypair,
            management_peer_id,
            builtins_management_peer_id,
        };

        this.load_persisted_keypairs_and_workers();
        this
    }

    fn load_persisted_keypairs_and_workers(&self) {
        let (keypairs, workers) = load_persisted_keypairs_and_workers(&self.keypairs_dir);

        for keypair in keypairs {
            let worker_id = keypair.get_peer_id();
            self.worker_keypairs.write().insert(worker_id, keypair);
        }

        for w in workers {
            let worker_id = w.worker_id;
            let deal_id = w.deal_id.clone();
            self.worker_infos
                .write()
                .insert(worker_id, Arc::new(w.into()));
            self.worker_ids.write().insert(deal_id, worker_id);
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

    pub async fn create_worker(
        &self,
        deal_id: String,
        init_peer_id: PeerId,
    ) -> Result<PeerId, KeyManagerError> {
        let worker_id = self.worker_ids.read().get(&deal_id).cloned();
        match worker_id {
            Some(_) => Err(WorkerAlreadyExists { deal_id }),
            _ => {
                let kp = KeyPair::generate_ed25519();
                let worker_id = kp.get_peer_id();

                self.store_keypair(worker_id, kp).await?;
                self.store_worker(worker_id, deal_id, init_peer_id).await?;
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

    pub fn list_workers(&self) -> Vec<WorkerId> {
        self.worker_infos.read().keys().cloned().collect()
    }

    pub fn get_deal_id(&self, worker_id: PeerId) -> Result<DealId, KeyManagerError> {
        self.get_worker_info(worker_id)
            .map(|info| info.deal_id.clone())
    }

    pub async fn remove_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        let deal_id = self.get_deal_id(worker_id)?;
        remove_keypair(&self.keypairs_dir, worker_id).await?;
        remove_worker(&self.keypairs_dir, worker_id).await?;
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
            self.get_worker_info(worker_id).map(|i| i.creator)
        }
    }

    async fn store_keypair(
        &self,
        worker_id: PeerId,
        keypair: KeyPair,
    ) -> Result<(), KeyManagerError> {
        persist_keypair(&self.keypairs_dir, worker_id, (&keypair).try_into()?).await?;
        self.worker_keypairs.write().insert(worker_id, keypair);

        Ok(())
    }

    async fn store_worker(
        &self,
        worker_id: PeerId,
        deal_id: String,
        creator: PeerId,
    ) -> Result<(), KeyManagerError> {
        let worker_info = Arc::new(WorkerInfo {
            deal_id: deal_id.clone(),
            creator,
            active: RwLock::new(true),
        });
        self.worker_infos.write().insert(worker_id, worker_info);
        self.worker_ids.write().insert(deal_id.clone(), worker_id);
        persist_worker(
            self.keypairs_dir.clone(),
            worker_id,
            PersistedWorker {
                worker_id,
                creator,
                deal_id,
                active: true,
            },
        )
        .await?;
        Ok(())
    }

    pub fn get_worker_info(&self, worker_id: PeerId) -> Result<Arc<WorkerInfo>, KeyManagerError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .cloned()
            .ok_or(WorkerNotFound(worker_id))
    }

    pub async fn activate_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        let keypairs_dir = self.keypairs_dir.clone();
        let worker_info = self.get_worker_info(worker_id)?;

        let task = tokio::task::Builder::new()
            .name(&format!("Worker activation {}", worker_id.to_base58()))
            .spawn_blocking(move || {
                let mut active = worker_info.active.write();
                *active = true;
                persist_worker_sync(
                    keypairs_dir,
                    worker_id,
                    PersistedWorker {
                        worker_id,
                        creator: worker_info.creator,
                        deal_id: worker_info.deal_id.clone(),
                        active: *active,
                    },
                )
            })
            .expect("Could not spawn 'Worker Activation' task");

        task.await.map_err(|_| KeyManagerError::InternalError)?
    }

    pub async fn deactivate_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        let keypairs_dir = self.keypairs_dir.clone();
        let worker_info = self.get_worker_info(worker_id)?;

        let task = tokio::task::Builder::new()
            .name(&format!("Worker deactivation {}", worker_id.to_base58()))
            .spawn_blocking(move || {
                let mut active = worker_info.active.write();
                *active = false;
                persist_worker_sync(
                    keypairs_dir,
                    worker_id,
                    PersistedWorker {
                        worker_id,
                        creator: worker_info.creator,
                        deal_id: worker_info.deal_id.clone(),
                        active: *active,
                    },
                )
            })
            .expect("Could not spawn 'Worker Deactivation' task");

        task.await.map_err(|_| KeyManagerError::InternalError)?
    }

    pub fn is_worker_active(&self, worker_id: PeerId) -> bool {
        // host is always active
        if self.is_host(worker_id) {
            return true;
        }

        let worker_info = self.worker_infos.read().get(&worker_id).cloned();

        match worker_info {
            Some(worker_info) => *worker_info.active.read(),
            None => {
                tracing::warn!(target = "key-manager", "Worker {} not found", worker_id);
                false
            }
        }
    }
}
