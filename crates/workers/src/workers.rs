use crate::error::WorkersError;
use crate::persistence::{load_persisted_workers, persist_worker, remove_worker, PersistedWorker};
use crate::scope::Scope;
use crate::{DealId, KeyStorage, WorkerId};
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
    scope: Scope,
}

impl Workers {
    pub fn new(key_storage: Arc<KeyStorage>, scope: Scope) -> Self {
        Self {
            worker_ids: Default::default(),
            worker_infos: Default::default(),
            workers_dir: Default::default(),
            key_storage,
            scope,
        }
    }

    pub async fn from_path(
        workers_dir: &Path,
        key_storage: Arc<KeyStorage>,
        scope: Scope,
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
            scope,
        })
    }

    pub async fn create_worker(
        &self,
        deal_id: String,
        init_peer_id: PeerId,
    ) -> Result<PeerId, WorkersError> {
        let worker_id = {
            let guard = self.worker_ids.read();
            guard.get(&deal_id).cloned()
        };
        match worker_id {
            Some(_) => Err(WorkersError::WorkerAlreadyExists { deal_id }),
            _ => {
                let key_pair = self
                    .key_storage
                    .create_key_pair()
                    .await
                    .map_err(|err| WorkersError::CreateWorkerKeyPair { err })?;

                let worker_id = key_pair.get_peer_id();

                let worker_info = self
                    .store_worker(worker_id, deal_id.clone(), init_peer_id)
                    .await;

                match worker_info {
                    Ok(worker_info) => {
                        let mut worker_ids = self.worker_ids.write();
                        let mut worker_infos = self.worker_infos.write();

                        if worker_ids.contains_key(&deal_id) {
                            return Err(WorkersError::WorkerAlreadyExists { deal_id });
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
                            .map_err(|err| WorkersError::RemoveWorkerKeyPair { err })?;

                        return Err(err);
                    }
                }

                Ok(worker_id)
            }
        }
    }

    pub fn get_worker_keypair(&self, worker_id: PeerId) -> Result<KeyPair, WorkersError> {
        if self.scope.is_host(worker_id) {
            Ok(self.key_storage.root_key_pair.clone())
        } else {
            self.key_storage
                .get_key_pair(worker_id)
                .ok_or(WorkersError::KeypairNotFound(worker_id))
        }
    }

    pub fn list_workers(&self) -> Vec<WorkerId> {
        self.worker_infos.read().keys().cloned().collect()
    }

    pub fn get_deal_id(&self, worker_id: PeerId) -> Result<DealId, WorkersError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .ok_or(WorkersError::WorkerNotFound(worker_id))
            .map(|info| info.deal_id.clone())
    }

    pub async fn remove_worker(&self, worker_id: PeerId) -> Result<(), WorkersError> {
        let deal_id = self.get_deal_id(worker_id)?;
        remove_worker(&self.workers_dir, worker_id).await?;
        self.key_storage
            .remove_key_pair(worker_id)
            .await
            .map_err(|err| WorkersError::RemoveWorkerKeyPair { err })?;

        let mut worker_ids = self.worker_ids.write();
        let mut worker_infos = self.worker_infos.write();
        let removed_worker_id = worker_ids.remove(&deal_id);
        let removed_worker_info = worker_infos.remove(&worker_id);

        debug_assert!(removed_worker_id.is_some(), "worker_id does not exist");
        debug_assert!(removed_worker_info.is_some(), "worker info does not exist");

        Ok(())
    }

    pub fn get_worker_id(&self, deal_id: String) -> Result<PeerId, WorkersError> {
        self.worker_ids
            .read()
            .get(&deal_id)
            .cloned()
            .ok_or(WorkersError::WorkerNotFoundByDeal(deal_id))
    }

    pub fn get_worker_creator(&self, worker_id: PeerId) -> Result<PeerId, WorkersError> {
        if self.scope.is_host(worker_id) {
            Ok(worker_id)
        } else {
            self.worker_infos
                .read()
                .get(&worker_id)
                .map(|i| i.creator)
                .ok_or(WorkersError::WorkerNotFound(worker_id))
        }
    }

    async fn store_worker(
        &self,
        worker_id: PeerId,
        deal_id: String,
        creator: PeerId,
    ) -> Result<WorkerInfo, WorkersError> {
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
        let worker_info = WorkerInfo {
            deal_id: deal_id.clone(),
            creator,
            active: RwLock::new(true),
        };
        Ok(worker_info)
    }

    async fn set_worker_status(&self, worker_id: PeerId, status: bool) -> Result<(), WorkersError> {
        let (creator, deal_id) = {
            let guard = self.worker_infos.read();
            let worker_info = guard
                .get(&worker_id)
                .ok_or(WorkersError::WorkerNotFound(worker_id))?;
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
    pub async fn activate_worker(&self, worker_id: PeerId) -> Result<(), WorkersError> {
        self.set_worker_status(worker_id, true).await?;
        Ok(())
    }

    pub async fn deactivate_worker(&self, worker_id: PeerId) -> Result<(), WorkersError> {
        self.set_worker_status(worker_id, false).await?;
        Ok(())
    }

    pub fn is_worker_active(&self, worker_id: PeerId) -> bool {
        // host is always active
        if self.scope.is_host(worker_id) {
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
                    "Worker {worker_id} not found"
                );
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{KeyStorage, Scope, Workers};
    use libp2p::PeerId;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_workers_creation() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let scope = Scope::new(
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        // Check that the workers instance has the correct initial state
        assert_eq!(workers.worker_ids.read().len(), 0);
        assert_eq!(workers.worker_infos.read().len(), 0);
        assert_eq!(workers.workers_dir, workers_dir);
    }

    #[tokio::test]
    async fn test_worker_creation() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let host_peer_id = PeerId::random();
        let scope = Scope::new(
            host_peer_id,
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        let creator_peer_id = PeerId::random();
        let worker_id = workers
            .create_worker("deal_id_1".to_string(), creator_peer_id)
            .await
            .expect("Failed to create worker");

        let deal_id = workers
            .get_deal_id(worker_id)
            .expect("Failed to get deal id");
        assert_eq!(deal_id, "deal_id_1".to_string());
        let key_pair_1 = key_storage.get_key_pair(worker_id);
        assert!(key_pair_1.is_some());
        assert_eq!(key_pair_1.clone().unwrap().get_peer_id(), worker_id);

        let key_pair_2 = workers
            .get_worker_keypair(worker_id)
            .expect("Failed to get deal id");

        assert_eq!(key_pair_1.unwrap().to_vec(), key_pair_2.to_vec());

        let list_workers = workers.list_workers();
        assert_eq!(list_workers, vec![worker_id]);

        let creator = workers
            .get_worker_creator(host_peer_id)
            .expect("Failed to get worker creator");
        assert_eq!(creator, host_peer_id);

        let creator = workers
            .get_worker_creator(worker_id)
            .expect("Failed to get worker creator");
        assert_eq!(creator, creator_peer_id);

        let worker_id_1 = workers
            .get_worker_id("deal_id_1".to_string())
            .expect("Failed to get worker id");
        assert_eq!(worker_id_1, worker_id);
    }

    #[tokio::test]
    async fn test_worker_creation_dupes() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let scope = Scope::new(
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        let worker_id = workers
            .create_worker("deal_id_1".to_string(), PeerId::random())
            .await
            .expect("Failed to create worker");

        let deal_id = workers
            .get_deal_id(worker_id)
            .expect("Failed to get deal id");
        assert_eq!(deal_id, "deal_id_1".to_string());

        let res = workers
            .create_worker("deal_id_1".to_string(), PeerId::random())
            .await;

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "Worker for deal_id_1 already exists"
        )
    }

    #[tokio::test]
    async fn test_worker_remove() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let scope = Scope::new(
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        let worker_id_1 = workers
            .create_worker("deal_id_1".to_string(), PeerId::random())
            .await
            .expect("Failed to create worker");

        let worker_id_2 = workers
            .create_worker("deal_id_2".to_string(), PeerId::random())
            .await
            .expect("Failed to create worker");

        let mut list = workers.list_workers();
        list.sort();
        let mut expected_list = vec![worker_id_1, worker_id_2];
        expected_list.sort();

        assert_eq!(list, expected_list);

        workers
            .remove_worker(worker_id_2)
            .await
            .expect("Failed to remove worker id");

        let list = workers.list_workers();
        let expected_list = vec![worker_id_1];

        assert_eq!(list, expected_list);
        let key_1 = key_storage.get_key_pair(worker_id_1);
        let key_2 = key_storage.get_key_pair(worker_id_2);
        assert!(key_1.is_some());
        assert!(key_2.is_none());
    }

    #[tokio::test]
    async fn test_persistence() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let scope = Scope::new(
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        let worker_id_1 = workers
            .create_worker("deal_id_1".to_string(), PeerId::random())
            .await
            .expect("Failed to create worker");

        let worker_id_2 = workers
            .create_worker("deal_id_2".to_string(), PeerId::random())
            .await
            .expect("Failed to create worker");

        let mut list = workers.list_workers();
        list.sort();
        let mut expected_list = vec![worker_id_1, worker_id_2];
        expected_list.sort();

        assert_eq!(list, expected_list);

        workers
            .remove_worker(worker_id_2)
            .await
            .expect("Failed to remove worker id");

        let list = workers.list_workers();
        let expected_list = vec![worker_id_1];

        assert_eq!(list, expected_list);
        let key_1 = key_storage.get_key_pair(worker_id_1);
        let key_2 = key_storage.get_key_pair(worker_id_2);
        assert!(key_1.is_some());
        assert!(key_2.is_none());
        let status = workers.is_worker_active(worker_id_1);
        assert!(status);
        workers
            .deactivate_worker(worker_id_1)
            .await
            .expect("Failed to activate worker");
        let status = workers.is_worker_active(worker_id_1);
        assert!(!status);
        drop(key_storage);
        drop(scope);
        drop(workers);

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(&key_pairs_dir, root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let scope = Scope::new(
            PeerId::random(),
            PeerId::random(),
            PeerId::random(),
            key_storage.clone(),
        ); // Customize with appropriate scope

        // Create a new Workers instance
        let workers = Workers::from_path(&workers_dir, key_storage.clone(), scope.clone())
            .await
            .expect("Failed to create Workers from path");

        let list = workers.list_workers();
        let expected_list = vec![worker_id_1];

        assert_eq!(list, expected_list);
        let key_1 = key_storage.get_key_pair(worker_id_1);
        let key_2 = key_storage.get_key_pair(worker_id_2);
        assert!(key_1.is_some());
        assert!(key_2.is_none());
        let status = workers.is_worker_active(worker_id_1);
        assert!(!status);
    }
}
