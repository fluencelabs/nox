use crate::error::WorkerRegistryError;
use crate::persistence::{
    persist_keypair, persist_worker, remove_keypair, remove_worker, PersistedWorker,
};
use crate::{DealId, KeyStorage, KeyManagerError, WorkerId};
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::security::Security;

pub struct WorkerInfo {
    pub deal_id: String,
    pub creator: PeerId,
    pub active: RwLock<bool>,
}

pub struct WorkerRegistry {
    /// deal_id -> worker_id
    worker_ids: Arc<RwLock<HashMap<DealId, WorkerId>>>,
    /// worker_id -> worker_info
    worker_infos: Arc<RwLock<HashMap<WorkerId, WorkerInfo>>>,

    workers_dir: PathBuf,

    key_manager: KeyStorage,
    security: Security
}

impl WorkerRegistry {
    pub fn new(key_manager: KeyStorage, security: Security) -> Self {
        Self {
            worker_ids: Arc::new(Default::default()),
            worker_infos: Arc::new(Default::default()),
            workers_dir: Default::default(),
            key_manager,
            security
        }
    }

    pub async fn from_path(workers_dir: &Path, key_manager: KeyStorage, security: Security) -> eyre::Result<Self> {
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
            worker_ids: Arc::new(RwLock::new(worker_ids)),
            worker_infos: Arc::new(RwLock::new(worker_infos)),
            workers_dir: workers_dir.to_path_buf(),
            key_manager,
            security
        })
    }

    pub fn create_worker(
        &self,
        deal_id: String,
        init_peer_id: PeerId,
    ) -> Result<PeerId, WorkerRegistryError> {
        let worker_id = self.worker_ids.read().get(&deal_id).cloned();
        match worker_id {
            Some(_) => Err(WorkerRegistryError::WorkerAlreadyExists { deal_id }),
            _ => {
                let mut worker_ids = self.worker_ids.write();
                let mut worker_infos = self.worker_infos.write();
                let mut worker_keypairs = self.worker_keypairs.write();

                if worker_ids.contains_key(&deal_id) {
                    return Err(WorkerRegistryError::WorkerAlreadyExists { deal_id });
                }

                self.key_manager.create_key_pair();

                let keypair = KeyPair::generate_ed25519();
                persist_keypair(&self.keypairs_dir, worker_id, (&keypair).try_into()?)?;

                match self.store_worker(worker_id, deal_id.clone(), init_peer_id) {
                    Ok(worker_info) => {
                        worker_keypairs.insert(worker_id, keypair);
                        worker_ids.insert(deal_id, worker_id);
                        worker_infos.insert(worker_id, worker_info);
                    }
                    Err(err) => {
                        tracing::warn!(
                            target = "key-manager",
                            "Failed to store worker info for {}: {}",
                            worker_id,
                            err
                        );

                        remove_keypair(&self.keypairs_dir, worker_id)?;
                        return Err(err);
                    }
                }

                Ok(worker_id)
            }
        }
    }

    pub fn get_worker_keypair(&self, worker_id: PeerId) -> Result<KeyPair, KeyManagerError> {
        if self.is_host(worker_id) {
            Ok(self.root_key_pair.clone())
        } else {
            self.key_manager
            self.worker_key_pairs
                .read()
                .get(&worker_id)
                .cloned()
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
    pub fn remove_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        let deal_id = self.get_deal_id(worker_id)?;
        let mut worker_ids = self.worker_ids.write();
        let mut worker_infos = self.worker_infos.write();
        let mut worker_keypairs = self.worker_keypairs.write();
        remove_keypair(&self.keypairs_dir, worker_id)?;
        remove_worker(&self.workers_dir, worker_id)?;
        let removed_worker_id = worker_ids.remove(&deal_id);
        let removed_worker_info = worker_infos.remove(&worker_id);
        let removed_worker_kp = worker_keypairs.remove(&worker_id);

        debug_assert!(removed_worker_id.is_some(), "worker_id does not exist");
        debug_assert!(removed_worker_info.is_some(), "worker info does not exist");
        debug_assert!(removed_worker_kp.is_some(), "worker kp does not exist");

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
        if self.is_host(worker_id) {
            Ok(worker_id)
        } else {
            self.worker_infos
                .read()
                .get(&worker_id)
                .map(|i| i.creator)
                .ok_or(WorkerRegistryError::WorkerNotFound(worker_id))
        }
    }

    fn store_worker(
        &self,
        worker_id: PeerId,
        deal_id: String,
        creator: PeerId,
    ) -> Result<WorkerInfo, KeyManagerError> {
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
        )?;
        Ok(worker_info)
    }

    fn set_worker_status(&self, worker_id: PeerId, status: bool) -> Result<(), KeyManagerError> {
        let guard = self.worker_infos.read();
        let worker_info = guard
            .get(&worker_id)
            .ok_or(WorkerRegistryError::WorkerNotFound(worker_id))?;

        let mut active = worker_info.active.write();
        *active = status;
        persist_worker(
            &self.workers_dir,
            worker_id,
            PersistedWorker {
                worker_id,
                creator: worker_info.creator,
                deal_id: worker_info.deal_id.clone(),
                active: *active,
            },
        )
    }
    pub fn activate_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        self.set_worker_status(worker_id, true)
    }

    pub fn deactivate_worker(&self, worker_id: PeerId) -> Result<(), KeyManagerError> {
        self.set_worker_status(worker_id, false)
    }

    pub fn is_worker_active(&self, worker_id: PeerId) -> bool {
        // host is always active
        if self.is_host(worker_id) {
            return true;
        }

        let guard = self.worker_infos.read();
        let worker_info = guard.get(&worker_id);

        match worker_info {
            Some(worker_info) => *worker_info.active.read(),
            None => {
                tracing::warn!(target = "key-manager", "Worker {} not found", worker_id);
                false
            }
        }
    }
}

/// Load info about persisted workers from disk
async fn load_persisted_workers(workers_dir: &Path) -> eyre::Result<Vec<PersistedWorker>> {
    let list_files = tokio::fs::read_dir(workers_dir).await.ok();

    let files = match list_files {
        Some(mut entries) => {
            let mut paths = vec![];
            while let Some(entry) = entries.next_entry().await? {
                paths.push(entry.path())
            }
            paths
        }
        None => {
            // Attempt to create directory
            tokio::fs::create_dir_all(workers_dir)
                .await
                .map_err(|err| WorkerRegistryError::CreateWorkersDir {
                    path: workers_dir.to_path_buf(),
                    err,
                })?;
            vec![]
        }
    };

    let mut workers = vec![];
    for file in files.iter() {
        let res: eyre::Result<()> = try {
            if crate::persistence::is_worker(file) {
                workers.push(crate::persistence::load_persisted_worker(file).await?);
            }
        };

        if let Err(err) = res {
            log::warn!("{err}")
        }
    }

    Ok(workers)
}
