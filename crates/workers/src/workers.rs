use crate::error::WorkerRegistryError;
use crate::persistence::{load_persisted_workers, persist_worker, remove_worker, PersistedWorker};
use crate::scope::Scopes;
use crate::{DealId, KeyManagerError, KeyStorage, WorkerId};
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct WorkerInfo {
    pub deal_id: String,
    pub creator: PeerId,
    pub active: RwLock<bool>,
}

pub struct Workers {
    /// deal_id -> worker_id
    worker_ids: RwLock<HashMap<DealId, WorkerId>>,
    /// worker_id -> worker_info
    worker_infos: RwLock<HashMap<WorkerId, WorkerInfo>>,
    workers_dir: PathBuf,

    key_storage: Arc<KeyStorage>,
    scopes: Scopes,
}

impl Workers {
    pub fn new(key_storage: Arc<KeyStorage>, scopes: Scopes) -> Self {
        Self {
            worker_ids: Default::default(),
            worker_infos: Default::default(),
            workers_dir: Default::default(),
            key_storage,
            scopes,
        }
    }

    pub async fn from_path(
        workers_dir: &Path,
        key_storage: Arc<KeyStorage>,
        scopes: Scopes,
    ) -> eyre::Result<Self> {
        let workers = load_persisted_workers(workers_dir).await?;
        let mut worker_ids = HashMap::with_capacity(workers.len());
        let mut worker_infos = HashMap::with_capacity(workers.len());

        for w in workers {
            let worker_id = w.worker_id;
            let deal_id = w.deal_id.clone();
            worker_infos.insert(worker_id, w.into());
            worker_ids.insert(deal_id, worker_id);
        }
        Ok(Self {
            worker_ids: RwLock::new(worker_ids),
            worker_infos: RwLock::new(worker_infos),
            workers_dir: workers_dir.to_path_buf(),
            key_storage,
            scopes,
        })
    }

    pub async fn create_worker(
        &self,
        deal_id: String,
        init_peer_id: PeerId,
    ) -> Result<PeerId, WorkerRegistryError> {
        let worker_id = {
            let guard = self.worker_ids.read();
            guard.get(&deal_id).cloned()
        };
        match worker_id {
            Some(_) => Err(WorkerRegistryError::WorkerAlreadyExists { deal_id }),
            _ => {
                let key_pair = self
                    .key_storage
                    .create_key_pair()
                    .await
                    .map_err(|err| WorkerRegistryError::CreateWorkerKeyPair { err })?;

                let worker_id = key_pair.get_peer_id();

                match self
                    .store_worker(worker_id, deal_id.clone(), init_peer_id)
                    .await
                {
                    Ok(worker_info) => {
                        let mut worker_ids = self.worker_ids.write();
                        let mut worker_infos = self.worker_infos.write();

                        if worker_ids.contains_key(&deal_id) {
                            return Err(WorkerRegistryError::WorkerAlreadyExists { deal_id });
                        }

                        worker_ids.insert(deal_id, worker_id);
                        worker_infos.insert(worker_id, worker_info);
                    }
                    Err(err) => {
                        tracing::warn!(
                            target = "worker-registry",
                            worker_id = worker_id.to_string(),
                            "Failed to store worker info for {worker_id}: {}",
                            err
                        );
                        self.key_storage
                            .remove_key_pair(worker_id)
                            .await
                            .map_err(|err| WorkerRegistryError::RemoveWorkerKeyPair { err })?;

                        return Err(err);
                    }
                }

                Ok(worker_id)
            }
        }
    }

    pub fn get_worker_keypair(&self, worker_id: PeerId) -> Result<KeyPair, KeyManagerError> {
        if self.scopes.is_host(worker_id) {
            Ok(self.key_storage.root_key_pair.clone())
        } else {
            self.key_storage
                .get_key_pair(worker_id)
                .ok_or(KeyManagerError::KeypairNotFound(worker_id))
        }
    }

    pub fn list_workers(&self) -> Vec<WorkerId> {
        self.worker_infos.read().keys().cloned().collect()
    }

    pub fn get_deal_id(&self, worker_id: PeerId) -> Result<DealId, WorkerRegistryError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .ok_or(WorkerRegistryError::WorkerNotFound(worker_id))
            .map(|info| info.deal_id.clone())
    }
    pub async fn remove_worker(&self, worker_id: PeerId) -> Result<(), WorkerRegistryError> {
        let deal_id = self.get_deal_id(worker_id)?;
        remove_worker(&self.workers_dir, worker_id).await?;
        self.key_storage
            .remove_key_pair(worker_id)
            .await
            .map_err(|err| WorkerRegistryError::RemoveWorkerKeyPair { err })?;

        let mut worker_ids = self.worker_ids.write();
        let mut worker_infos = self.worker_infos.write();
        let removed_worker_id = worker_ids.remove(&deal_id);
        let removed_worker_info = worker_infos.remove(&worker_id);

        debug_assert!(removed_worker_id.is_some(), "worker_id does not exist");
        debug_assert!(removed_worker_info.is_some(), "worker info does not exist");

        Ok(())
    }

    pub fn get_worker_id(&self, deal_id: String) -> Result<PeerId, WorkerRegistryError> {
        self.worker_ids
            .read()
            .get(&deal_id)
            .cloned()
            .ok_or(WorkerRegistryError::WorkerNotFoundByDeal(deal_id))
    }

    pub fn get_worker_creator(&self, worker_id: PeerId) -> Result<PeerId, WorkerRegistryError> {
        if self.scopes.is_host(worker_id) {
            Ok(worker_id)
        } else {
            self.worker_infos
                .read()
                .get(&worker_id)
                .map(|i| i.creator)
                .ok_or(WorkerRegistryError::WorkerNotFound(worker_id))
        }
    }

    async fn store_worker(
        &self,
        worker_id: PeerId,
        deal_id: String,
        creator: PeerId,
    ) -> Result<WorkerInfo, WorkerRegistryError> {
        let worker_info = WorkerInfo {
            deal_id: deal_id.clone(),
            creator,
            active: RwLock::new(true),
        };

        persist_worker(
            &self.workers_dir,
            worker_id,
            PersistedWorker {
                worker_id,
                creator,
                deal_id,
                active: true,
            },
        )
        .await?;
        Ok(worker_info)
    }

    async fn set_worker_status(
        &self,
        worker_id: PeerId,
        status: bool,
    ) -> Result<(), WorkerRegistryError> {
        let (creator, deal_id) = {
            let guard = self.worker_infos.read();
            let worker_info = guard
                .get(&worker_id)
                .ok_or(WorkerRegistryError::WorkerNotFound(worker_id))?;
            let mut active = worker_info.active.write();
            *active = status;

            (worker_info.creator, worker_info.deal_id.clone())
        };

        persist_worker(
            &self.workers_dir,
            worker_id,
            PersistedWorker {
                worker_id,
                creator,
                deal_id,
                active: status,
            },
        )
        .await?;
        Ok(())
    }
    pub async fn activate_worker(&self, worker_id: PeerId) -> Result<(), WorkerRegistryError> {
        self.set_worker_status(worker_id, true).await?;
        Ok(())
    }

    pub async fn deactivate_worker(&self, worker_id: PeerId) -> Result<(), WorkerRegistryError> {
        self.set_worker_status(worker_id, false).await?;
        Ok(())
    }

    pub fn is_worker_active(&self, worker_id: PeerId) -> bool {
        // host is always active
        if self.scopes.is_host(worker_id) {
            return true;
        }

        let guard = self.worker_infos.read();
        let worker_info = guard.get(&worker_id);

        match worker_info {
            Some(worker_info) => *worker_info.active.read(),
            None => {
                tracing::warn!(
                    target = "worker-registry",
                    worker_id = worker_id.to_string(),
                    "Worker {worker_id} not found");
                false
            }
        }
    }
}
