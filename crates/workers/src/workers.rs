/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::lock_api::RwLockUpgradableReadGuard;
use parking_lot::RwLock;
use tokio::runtime::{Handle, Runtime, UnhandledPanic};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::error::WorkersError;
use crate::persistence::{load_persisted_workers, persist_worker, remove_worker, PersistedWorker};
use crate::KeyStorage;
use core_distributor::types::{AcquireRequest, Assignment, WorkType};
use core_distributor::{CoreDistributor, ThreadPinner, CUID};
use fluence_libp2p::PeerId;
use types::peer_scope::WorkerId;
use types::DealId;
use vm_network_utils::NetworkSettings;
use vm_utils::{CreateVMDomainParams, NonEmpty, VmStatus};

const WORKER_DATA_DIR: &str = "data";

/// Information about a worker.
pub struct WorkerInfo {
    /// The unique identifier for the deal associated with the worker.
    pub deal_id: DealId,
    /// The ID of the peer that created the worker.
    pub creator: PeerId,
    /// A read-write lock indicating whether the worker is active.
    pub active: RwLock<bool>,
    /// A count of compute units available for this worker.
    pub cu_ids: Vec<CUID>,
    /// A read-write lock indicating that worker has a VM.
    pub vm_flag: RwLock<bool>,
}

pub struct WorkerParams {
    deal_id: DealId,
    creator: PeerId,
    cu_ids: Vec<CUID>,
}

impl WorkerParams {
    pub fn new(deal_id: DealId, creator: PeerId, cu_ids: Vec<CUID>) -> Self {
        Self {
            deal_id,
            creator,
            cu_ids,
        }
    }
}

/// Manages a collection of workers.
pub struct Workers {
    /// Manages a collection of workers.
    worker_ids: RwLock<HashMap<DealId, WorkerId>>,
    /// Mapping of worker IDs to worker information.
    worker_infos: RwLock<HashMap<WorkerId, WorkerInfo>>,
    /// Directory path where worker data is persisted.
    workers_dir: PathBuf,
    /// Key storage for managing worker key pairs.
    key_storage: Arc<KeyStorage>,
    /// Mapping of worker IDs to worker runtime.
    runtimes: RwLock<HashMap<WorkerId, Runtime>>,
    /// Core distributor for core assignment
    core_distributor: Arc<dyn CoreDistributor>,
    /// Core pinning helper
    thread_pinner: Arc<dyn ThreadPinner>,
    /// Number of created tokio runtimes
    runtime_counter: Arc<AtomicU32>,

    assignments: RwLock<HashMap<WorkerId, Assignment>>,

    sender: Sender<Event>,

    config: WorkersConfig,
}

#[derive(Debug)]
pub enum Event {
    WorkerCreated {
        worker_id: WorkerId,
        thread_count: usize,
    },
    WorkerRemoved {
        worker_id: WorkerId,
    },
}

pub struct WorkersConfig {
    /// The notification channel size
    channel_size: usize,
    vm: Option<VmConfig>,
}

impl WorkersConfig {
    pub fn new(channel_size: usize, vm: Option<VmConfig>) -> Self {
        Self { channel_size, vm }
    }
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Uri to the libvirt API
    pub libvirt_uri: String,
    pub allow_gpu: bool,
    pub network: NetworkSettings,
}

impl VmConfig {
    pub fn new(libvirt_uri: String, allow_gpu: bool, network: NetworkSettings) -> Self {
        Self {
            libvirt_uri,
            allow_gpu,
            network,
        }
    }
}

impl Workers {
    /// Creates a `Workers` instance by loading persisted worker data from the specified directory.
    ///
    /// # Arguments
    ///
    /// * `workers_dir` - The path to the directory containing persisted worker data.
    /// * `key_storage` - An `Arc<KeyStorage>` instance for managing worker key pairs.
    /// * `scope` - A `Scope` instance used to determine the host and manage key pairs.
    ///
    /// # Returns
    ///
    /// Returns `Result<Self, eyre::Error>` where:
    /// - `Ok(workers)` if the `Workers` instance is successfully created.
    /// - `Err(eyre::Error)` if an error occurs during the creation process.
    ///
    pub async fn from_path(
        config: WorkersConfig,
        workers_dir: PathBuf,
        key_storage: Arc<KeyStorage>,
        core_distributor: Arc<dyn CoreDistributor>,
        thread_pinner: Arc<dyn ThreadPinner>,
    ) -> eyre::Result<(Self, Receiver<Event>)> {
        let workers = load_persisted_workers(workers_dir.as_path()).await?;
        let mut worker_ids = HashMap::with_capacity(workers.len());
        let mut worker_infos = HashMap::with_capacity(workers.len());
        let mut runtimes = HashMap::with_capacity(workers.len());
        let mut assignments = HashMap::with_capacity(workers.len());

        let worker_counter = Arc::new(AtomicU32::new(0));
        let (sender, receiver) = tokio::sync::mpsc::channel::<Event>(config.channel_size);

        for w in workers {
            let worker_id = w.worker_id;
            let deal_id = w.deal_id.clone().into();
            let cu_ids = w.cu_ids.clone();
            worker_infos.insert(worker_id, w.into());
            worker_ids.insert(deal_id, worker_id);

            let (runtime, thread_count, assignment) = Self::build_runtime(
                core_distributor.clone(),
                thread_pinner.clone(),
                worker_counter.clone(),
                worker_id,
                cu_ids,
            )?;
            assignments.insert(worker_id, assignment);
            runtimes.insert(worker_id, runtime);
            sender
                .send(Event::WorkerCreated {
                    worker_id,
                    thread_count,
                })
                .await?
        }
        Ok((
            Self {
                worker_ids: worker_ids.into(),
                worker_infos: worker_infos.into(),
                workers_dir,
                key_storage,
                runtimes: RwLock::new(runtimes),
                runtime_counter: worker_counter,
                core_distributor,
                thread_pinner,
                sender,
                assignments: assignments.into(),
                config,
            },
            receiver,
        ))
    }

    /// Retrieves the deal ID associated with the specified worker ID.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker for which the deal ID is requested.
    ///
    /// # Returns
    ///
    /// Returns `Result<DealId, WorkersError>` where:
    /// - `Ok(deal_id)` if the deal ID is successfully retrieved.
    /// - `Err(WorkersError)` if an error occurs, such as the worker not found.
    ///
    pub fn get_deal_id(&self, worker_id: WorkerId) -> Result<DealId, WorkersError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .ok_or(WorkersError::WorkerNotFound(worker_id))
            .map(|info| info.deal_id.clone())
    }

    /// Creates a new worker with the given `deal_id` and initial peer ID.
    ///
    /// # Arguments
    ///
    /// * `deal_id` - A `String` representing the unique identifier for the deal associated with the worker.
    /// * `init_peer_id` - The initial `PeerId` of the worker.
    ///
    /// # Returns
    ///
    /// Returns `Result<PeerId, WorkersError>` where:
    /// - `Ok(worker_id)` if the worker is successfully created, returning the ID of the created worker.
    /// - `Err(WorkersError)` if an error occurs, such as the worker already existing or key pair creation failure.
    ///
    pub async fn create_worker(&self, params: WorkerParams) -> Result<WorkerId, WorkersError> {
        let deal_id = params.deal_id;
        let init_peer_id = params.creator;
        let cu_ids = params.cu_ids;

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

                let worker_id: WorkerId = key_pair.get_peer_id().into();

                let worker_info = self
                    .store_worker(worker_id, deal_id.clone(), init_peer_id, cu_ids.clone())
                    .await;

                match worker_info {
                    Ok(worker_info) => {
                        let result: Result<(), WorkersError> = try {
                            let thread_count = {
                                let lock = self.worker_ids.upgradable_read();
                                let worker_ids = lock.deref();
                                if worker_ids.contains_key(&deal_id) {
                                    return Err(WorkersError::WorkerAlreadyExists { deal_id });
                                }

                                let (runtime, thread_count, assignment) = Self::build_runtime(
                                    self.core_distributor.clone(),
                                    self.thread_pinner.clone(),
                                    self.runtime_counter.clone(),
                                    worker_id,
                                    cu_ids.clone(),
                                )?;

                                // Upgrade read lock to write lock
                                let mut worker_ids = RwLockUpgradableReadGuard::upgrade(lock);
                                let mut worker_infos = self.worker_infos.write();
                                let mut runtimes = self.runtimes.write();
                                let mut worker_assignments = self.assignments.write();

                                worker_ids.insert(deal_id.clone(), worker_id);
                                worker_infos.insert(worker_id, worker_info);
                                runtimes.insert(worker_id, runtime);
                                worker_assignments.insert(worker_id, assignment);

                                thread_count
                            };

                            self.sender
                                .send(Event::WorkerCreated {
                                    worker_id,
                                    thread_count,
                                })
                                .await
                                .map_err(|_err| WorkersError::FailedToNotifySubsystem {
                                    worker_id,
                                })?
                        };
                        match result {
                            Ok(_) => {
                                tracing::info!(
                                    target = "worker-registry",
                                    worker_id = worker_id.to_string(),
                                    "Worker created {worker_id}"
                                );
                                Ok(())
                            }
                            Err(err) => {
                                tracing::error!(
                                    target = "worker-registry",
                                    worker_id = worker_id.to_string(),
                                    "Failed to notify subsystem for {worker_id}: {}",
                                    err
                                );
                                let mut worker_ids = self.worker_ids.write();
                                let mut worker_infos = self.worker_infos.write();
                                let mut runtimes = self.runtimes.write();

                                worker_ids.remove(&deal_id);
                                worker_infos.remove(&worker_id);
                                runtimes.remove(&worker_id);

                                self.core_distributor
                                    .release_worker_cores(cu_ids.as_slice());

                                Err(err)
                            }
                        }?
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

    /// Removes a worker with the specified `worker_id`.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker to be removed.
    ///
    /// # Returns
    ///
    /// Returns `Result<(), WorkersError>` where:
    /// - `Ok(())` if the worker is successfully removed.
    /// - `Err(WorkersError)` if an error occurs, such as the worker not found or key pair removal failure.
    ///
    pub async fn remove_worker(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        let deal_id = self.get_deal_id(worker_id)?;
        self.sender
            .send(Event::WorkerRemoved { worker_id })
            .await
            .map_err(|_err| WorkersError::FailedToNotifySubsystem { worker_id })?;

        self.remove_vm(worker_id)?;

        remove_worker(&self.workers_dir, worker_id).await?;

        self.key_storage
            .remove_key_pair(worker_id)
            .await
            .map_err(|err| WorkersError::RemoveWorkerKeyPair { err })?;

        let removed_runtime = {
            let mut worker_ids = self.worker_ids.write();
            let mut worker_infos = self.worker_infos.write();
            let mut runtimes = self.runtimes.write();
            let mut assignments = self.assignments.write();
            let removed_worker_id = worker_ids.remove(&deal_id);
            let removed_worker_info = worker_infos.remove(&worker_id);
            let removed_runtime = runtimes.remove(&worker_id);
            let removed_assignments = assignments.remove(&worker_id);

            debug_assert!(removed_worker_id.is_some(), "worker_id does not exist");
            debug_assert!(removed_worker_info.is_some(), "worker info does not exist");
            debug_assert!(removed_runtime.is_some(), "worker runtime does not exist");
            debug_assert!(
                removed_assignments.is_some(),
                "worker assignment does not exist"
            );
            removed_runtime
        };

        if let Some(runtime) = removed_runtime {
            // we can't shutdown the runtime in the async context, shift it to the blocking pool
            // also we don't wait the result
            tokio::task::Builder::new()
                .name(&format!("runtime-shutdown-{}", worker_id))
                .spawn_blocking(move || runtime.shutdown_background())
                .expect("Could not spawn task");
        }

        Ok(())
    }

    /// Activates the worker with the specified `worker_id`.
    ///
    /// The activation process sets the worker's status to `true`, indicating that the worker
    /// is active. The updated status is persisted, and internal data structures are updated.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker to be activated.
    ///
    /// # Returns
    ///
    /// Returns `Result<(), WorkersError>` where:
    /// - `Ok(())` if the activation is successful.
    /// - `Err(WorkersError)` if an error occurs during the activation process.
    ///
    pub async fn activate_worker(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        self.resume_vm(worker_id)?;
        self.set_worker_status(worker_id, true).await?;
        Ok(())
    }

    /// Deactivates the worker with the specified `worker_id`.
    ///
    /// The deactivation process sets the worker's status to `false`, indicating that the worker
    /// is not active. The updated status is persisted, and internal data structures are updated.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker to be deactivated.
    ///
    /// # Returns
    ///
    /// Returns `Result<(), WorkersError>` where:
    /// - `Ok(())` if the deactivation is successful.
    /// - `Err(WorkersError)` if an error occurs during the deactivation process.
    ///
    pub async fn deactivate_worker(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        self.pause_vm(worker_id)?;
        self.set_worker_status(worker_id, false).await?;
        Ok(())
    }

    pub fn get_runtime_handle(&self, worker_id: WorkerId) -> Option<Handle> {
        self.runtimes
            .read()
            .get(&worker_id)
            .map(|x| x.handle().clone())
    }

    /// Retrieves the creator `PeerId` associated with the specified worker `PeerId`.
    ///
    /// If the provided `worker_id` belongs to the host, the host's `PeerId` is returned.
    /// Otherwise, the creator's `PeerId` associated with the worker is retrieved.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker for which the creator `PeerId` is requested.
    ///
    /// # Returns
    ///
    /// Returns `Result<PeerId, WorkersError>` where:
    /// - `Ok(creator_peer_id)` if the creator `PeerId` is successfully retrieved.
    /// - `Err(WorkersError)` if an error occurs, such as the worker not found.
    ///
    pub fn get_worker_creator(&self, worker_id: WorkerId) -> Result<PeerId, WorkersError> {
        self.worker_infos
            .read()
            .get(&worker_id)
            .map(|i| i.creator)
            .ok_or(WorkersError::WorkerNotFound(worker_id))
    }

    /// Retrieves the worker ID associated with the specified `deal_id`.
    ///
    /// # Arguments
    ///
    /// * `deal_id` - The unique identifier (`String`) of the deal associated with the worker.
    ///
    /// # Returns
    ///
    /// Returns `Result<PeerId, WorkersError>` where:
    /// - `Ok(worker_id)` if the worker ID is successfully retrieved.
    /// - `Err(WorkersError)` if an error occurs, such as the worker not found.
    ///
    pub fn get_worker_id(&self, deal_id: DealId) -> Result<WorkerId, WorkersError> {
        self.worker_ids
            .read()
            .get(&deal_id)
            .cloned()
            .ok_or(WorkersError::WorkerNotFoundByDeal(deal_id))
    }

    /// Checks the activation status of the worker with the specified `worker_id`.
    ///
    /// The activation status indicates whether the worker is currently active or not. If the
    /// worker is the host, it is always considered active. Otherwise, the method checks the
    /// internal data structures to determine the worker's status.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker to check for activation status.
    ///
    /// # Returns
    ///
    /// Returns `true` if the worker is active, and `false` otherwise. If the worker with the
    /// specified `worker_id` is not found, a warning is logged, and `false` is returned.
    ///
    pub fn is_worker_active(&self, worker_id: WorkerId) -> bool {
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

    /// Retrieves a list of all worker IDs.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<WorkerId>` representing a list of all worker IDs currently registered.
    ///
    pub fn list_workers(&self) -> Vec<WorkerId> {
        self.worker_infos.read().keys().cloned().collect()
    }

    pub fn shutdown(&self) {
        tracing::debug!("Shutdown worker runtimes");
        let mut runtimes = self.runtimes.write();
        let mut deleted_runtimes = Vec::with_capacity(runtimes.len());
        let worker_ids: Vec<WorkerId> = runtimes.keys().cloned().collect();
        for worker_id in worker_ids {
            if let Some(runtime) = runtimes.remove(&worker_id) {
                deleted_runtimes.push(runtime);
            }
        }

        tokio::task::Builder::new()
            .name("workers-shutdown")
            .spawn_blocking(move || {
                for runtime in deleted_runtimes {
                    runtime.shutdown_background();
                }
            })
            .expect("Could not spawn a task");
    }

    /// Persists worker information and updates internal data structures.
    ///
    /// This method stores information about the worker identified by `worker_id` and associates
    /// it with the provided `deal_id` and `creator` PeerId. The worker's active status is set to `true`.
    /// The information is persisted to the workers' storage directory, and the internal
    /// `worker_ids` and `worker_infos` data structures are updated accordingly.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker to be stored.
    /// * `deal_id` - The unique identifier (`String`) associated with the deal.
    /// * `creator` - The `PeerId` of the creator of the worker.
    ///
    /// # Returns
    ///
    /// Returns `Result<WorkerInfo, WorkersError>` where:
    /// - `Ok(worker_info)` if the worker information is successfully stored.
    /// - `Err(WorkersError)` if an error occurs during the storage process.
    ///
    async fn store_worker(
        &self,
        worker_id: WorkerId,
        deal_id: DealId,
        creator: PeerId,
        cu_ids: Vec<CUID>,
    ) -> Result<WorkerInfo, WorkersError> {
        let worker_path = self.workers_dir.join(worker_id.to_string());

        tokio::fs::create_dir_all(&worker_path)
            .await
            .map_err(|err| WorkersError::WorkerStorageDirectory {
                path: worker_path.clone(),
                worker_id,
                err,
            })?;

        let worker_data_path = worker_path.join(WORKER_DATA_DIR);

        tokio::fs::create_dir_all(&worker_data_path)
            .await
            .map_err(|err| WorkersError::WorkerStorageDirectory {
                path: worker_data_path,
                worker_id,
                err,
            })?;

        persist_worker(
            &worker_path,
            PersistedWorker {
                worker_id,
                creator,
                deal_id: deal_id.clone().into(),
                active: true,
                cu_ids: cu_ids.clone(),
                vm_flag: false,
            },
        )
        .await?;
        let worker_info = WorkerInfo {
            deal_id,
            creator,
            active: RwLock::new(true),
            cu_ids,
            vm_flag: false.into(),
        };
        Ok(worker_info)
    }

    pub async fn create_vm(
        &self,
        worker_id: WorkerId,
        image: &Path,
    ) -> Result<String, WorkersError> {
        match &self.config.vm {
            None => Err(WorkersError::FeatureDisabled),
            Some(vm_config) => {
                let vm_created = self
                    .worker_infos
                    .read()
                    .values()
                    .any(|worker_info| *worker_info.vm_flag.read());
                if vm_created {
                    return Err(WorkersError::VMAlreadyExists(worker_id));
                }

                let vm_name = self.create_vm_inner(worker_id, image, vm_config).await?;
                Ok(vm_name)
            }
        }
    }

    async fn create_vm_inner(
        &self,
        worker_id: WorkerId,
        image: &Path,
        vm_config: &VmConfig,
    ) -> Result<String, WorkersError> {
        let (cores_count, assignment) = {
            let guard = self.assignments.read();
            let assignment = guard
                .get(&worker_id)
                .ok_or_else(|| WorkersError::WorkerNotFound(worker_id))?;

            let logical_cores = NonEmpty::from_vec(assignment.logical_core_ids())
                .ok_or_else(|| WorkersError::WrongAssignment)?;
            let physical_cores_count = assignment.physical_core_count();
            (physical_cores_count, logical_cores)
        };

        let file_name = &image
            .file_name()
            .ok_or_else(|| WorkersError::VMImageNotFile {
                image: image.to_path_buf(),
            })?;

        let vm_name = worker_id.to_string();

        let worker_image = self
            .workers_dir
            .join(&vm_name)
            .join(WORKER_DATA_DIR)
            .join(file_name);

        tokio::fs::copy(&image, &worker_image)
            .await
            .map_err(|err| WorkersError::FailedToCopyVMImage {
                image: image.to_path_buf(),
                err,
            })?;

        let params = CreateVMDomainParams::new(
            vm_name.clone(),
            worker_image,
            assignment,
            cores_count,
            vm_config.network.bridge_name.clone(),
            vm_config.allow_gpu,
        );

        vm_utils::create_domain(vm_config.libvirt_uri.clone().as_str(), &params)?;

        let result: Result<_, WorkersError> = try {
            // First, create the network
            vm_network_utils::setup_network(&vm_config.network, vm_name.as_str())?;
            // And only then start the VM
            vm_utils::start_vm(vm_config.libvirt_uri.as_str(), vm_name.as_str())?;
        };
        if let Err(err) = result {
            // Clear the network on errors, so we can retry later (setup_network isn't idempotent)
            tracing::warn!("couldn't create network or start VM, cleaning up network: {err}");
            vm_network_utils::clear_network(&vm_config.network, vm_name.as_str())?;
            return Err(err);
        }

        self.set_vm_flag(worker_id, true).await?;

        Ok(vm_name)
    }

    fn remove_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        if let Some(vm_config) = &self.config.vm {
            if self.has_vm(worker_id)? {
                vm_network_utils::clear_network(&vm_config.network, &worker_id.to_string())?;
                vm_utils::remove_domain(
                    vm_config.libvirt_uri.as_str(),
                    worker_id.to_string().as_str(),
                )?;
            }
        }
        Ok(())
    }

    pub fn start_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        if let Some(vm_config) = &self.config.vm {
            if self.has_vm(worker_id)? {
                vm_utils::start_vm(vm_config.libvirt_uri.as_str(), &worker_id.to_string())?;
            } else {
                return Err(WorkersError::VmNotFound(worker_id));
            }
        }
        Ok(())
    }

    pub fn stop_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        self.on_vm(worker_id, vm_utils::stop_vm)
    }

    pub fn reboot_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        self.on_vm(worker_id, vm_utils::reboot_vm)
    }

    pub fn reset_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        self.on_vm(worker_id, vm_utils::reset_vm)
    }

    fn pause_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        if let Some(vm_config) = &self.config.vm {
            if self.has_vm(worker_id)? {
                vm_utils::pause_vm(vm_config.libvirt_uri.as_str(), &worker_id.to_string())?;
            }
        }
        Ok(())
    }

    fn resume_vm(&self, worker_id: WorkerId) -> Result<(), WorkersError> {
        if let Some(vm_config) = &self.config.vm {
            if self.has_vm(worker_id)? {
                vm_utils::resume_vm(vm_config.libvirt_uri.as_str(), &worker_id.to_string())?;
            }
        }
        Ok(())
    }

    pub fn status_vm(&self, worker_id: WorkerId) -> Result<VmStatus, WorkersError> {
        self.on_vm(worker_id, vm_utils::status_vm)
    }

    fn on_vm<F, R>(&self, worker_id: WorkerId, f: F) -> Result<R, WorkersError>
    where
        F: Fn(&str, &str) -> Result<R, vm_utils::VmError>,
    {
        match &self.config.vm {
            Some(vm_config) => {
                if self.has_vm(worker_id)? {
                    let result = f(vm_config.libvirt_uri.as_str(), &worker_id.to_string())?;
                    Ok(result)
                } else {
                    Err(WorkersError::VmNotFound(worker_id))
                }
            }
            None => Err(WorkersError::FeatureDisabled),
        }
    }

    fn has_vm(&self, worker_id: WorkerId) -> Result<bool, WorkersError> {
        let guard = self.worker_infos.read();
        let worker_info = guard
            .get(&worker_id)
            .ok_or_else(|| WorkersError::WorkerNotFound(worker_id))?;
        let flag = *worker_info.vm_flag.read();
        Ok(flag)
    }

    async fn set_worker_info<F>(&self, worker_id: WorkerId, modify: F) -> Result<(), WorkersError>
    where
        F: Fn(&WorkerInfo),
    {
        let persisted_worker = {
            let guard = self.worker_infos.read();
            let worker_info = guard
                .get(&worker_id)
                .ok_or_else(|| WorkersError::WorkerNotFound(worker_id))?;

            modify(worker_info);

            let active = worker_info.active.read();
            let vm_flag = worker_info.vm_flag.read();

            PersistedWorker {
                worker_id,
                creator: worker_info.creator,
                deal_id: worker_info.deal_id.to_string(),
                active: *active,
                cu_ids: worker_info.cu_ids.clone(),
                vm_flag: *vm_flag,
            }
        };
        let worker_path = self.workers_dir.join(worker_id.to_string());
        persist_worker(&worker_path, persisted_worker).await?;
        Ok(())
    }

    async fn set_worker_status(
        &self,
        worker_id: WorkerId,
        status: bool,
    ) -> Result<(), WorkersError> {
        self.set_worker_info(worker_id, |worker_info: &WorkerInfo| {
            let mut active = worker_info.active.write();
            *active = status;
        })
        .await?;
        Ok(())
    }

    async fn set_vm_flag(&self, worker_id: WorkerId, value: bool) -> Result<(), WorkersError> {
        self.set_worker_info(worker_id, |worker_info: &WorkerInfo| {
            let mut vm_flag = worker_info.vm_flag.write();
            *vm_flag = value;
        })
        .await?;
        Ok(())
    }

    /// Retrieves the assignment associated with the specified worker `PeerId`.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The `PeerId` of the worker for which the creator `PeerId` is requested.
    ///
    /// # Returns
    ///
    /// Returns `Result<Assignment, WorkersError>` where:
    /// - `Ok(assignment)` if the creator `PeerId` is successfully retrieved.
    /// - `Err(WorkersError)` if an error occurs, such as the worker not found.
    ///
    pub fn get_worker_assignment(&self, worker_id: WorkerId) -> Result<Assignment, WorkersError> {
        self.assignments
            .read()
            .get(&worker_id)
            .cloned()
            .ok_or(WorkersError::WorkerNotFound(worker_id))
    }

    fn build_runtime(
        core_distributor: Arc<dyn CoreDistributor>,
        thread_pinner: Arc<dyn ThreadPinner>,
        worker_counter: Arc<AtomicU32>,
        worker_id: WorkerId,
        cu_ids: Vec<CUID>,
    ) -> Result<(Runtime, usize, Assignment), WorkersError> {
        // Creating a multi-threaded Tokio runtime with a total of cu_count * 2 threads.
        // We assume cu_count threads per logical processor, aligning with the common practice.
        let assignment = core_distributor
            .acquire_worker_cores(AcquireRequest::new(cu_ids, WorkType::Deal))
            .map_err(|err| WorkersError::FailedToAssignCores { worker_id, err })?;

        let logical_cores = assignment.logical_core_ids();
        let threads_count = logical_cores.len();

        let id = worker_counter.fetch_add(1, Ordering::Acquire);

        tracing::info!(target: "worker", "Creating runtime with id {} for worker id {}. Pinned to cores: {:?}", id, worker_id, logical_cores);

        let runtime_assignment = assignment.clone();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .thread_name(format!("worker-pool-{}", id))
            // Configuring worker threads for executing service calls and particles
            .worker_threads(threads_count)
            // Configuring blocking threads for handling I/O
            .max_blocking_threads(threads_count)
            .enable_time()
            .enable_io()
            .on_thread_start(move || {
                runtime_assignment.pin_current_thread_with(thread_pinner.as_ref());
            })
            .unhandled_panic(UnhandledPanic::Ignore) // TODO: try to log panics after fix https://github.com/tokio-rs/tokio/issues/4516
            .build()
            .map_err(|err| WorkersError::CreateRuntime { worker_id, err })?;
        Ok((runtime, threads_count, assignment))
    }
}

#[cfg(test)]
mod tests {
    use crate::{KeyStorage, WorkerParams, Workers, WorkersConfig, CUID};
    use core_distributor::dummy::DummyCoreDistibutor;
    use fs_utils::create_dirs;
    use hex::FromHex;
    use libp2p::PeerId;
    use std::sync::Arc;
    use tempfile::tempdir;
    use types::peer_scope::PeerScope;

    #[tokio::test]
    async fn test_workers_creation() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();
        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        create_dirs(&[&workers_dir]).unwrap();

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );

        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor,
            thread_pinner,
        )
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
        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        create_dirs(&[&workers_dir]).unwrap();

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);
        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );

        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor,
            thread_pinner,
        )
        .await
        .expect("Failed to create Workers from path");

        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let unit_ids = vec![init_id_1];

        let creator_peer_id = PeerId::random();
        let worker_id = workers
            .create_worker(WorkerParams::new(
                "deal_id_1".into(),
                creator_peer_id,
                unit_ids,
            ))
            .await
            .expect("Failed to create worker");

        let deal_id = workers
            .get_deal_id(worker_id)
            .expect("Failed to get deal id");
        assert_eq!(deal_id, "deal_id_1".to_string());
        let key_pair_1 = key_storage.get_worker_key_pair(worker_id);
        assert!(key_pair_1.is_some());
        assert_eq!(key_pair_1.clone().unwrap().get_peer_id(), worker_id.into());

        let key_pair_2 = key_storage
            .get_keypair(PeerScope::WorkerId(worker_id))
            .expect("Failed to get deal id");

        assert_eq!(key_pair_1.unwrap().to_vec(), key_pair_2.to_vec());

        let list_workers = workers.list_workers();
        assert_eq!(list_workers, vec![worker_id]);

        let creator = workers
            .get_worker_creator(worker_id)
            .expect("Failed to get worker creator");
        assert_eq!(creator, creator_peer_id);

        let worker_id_1 = workers
            .get_worker_id("deal_id_1".into())
            .expect("Failed to get worker id");
        assert_eq!(worker_id_1, worker_id);
        // tokio doesn't allow to drop runtimes in async context, so shifting workers drop to the blocking thread
        tokio::task::spawn_blocking(|| drop(workers)).await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_creation_dupes() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();
        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        create_dirs(&[&workers_dir]).unwrap();

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);
        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor,
            thread_pinner,
        )
        .await
        .expect("Failed to create Workers from path");

        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let unit_ids = vec![init_id_1];

        let worker_id = workers
            .create_worker(WorkerParams::new(
                "deal_id_1".into(),
                PeerId::random(),
                unit_ids.clone(),
            ))
            .await
            .expect("Failed to create worker");

        let deal_id = workers
            .get_deal_id(worker_id)
            .expect("Failed to get deal id");
        assert_eq!(deal_id, "deal_id_1".to_string());

        let res = workers
            .create_worker(WorkerParams::new(
                "deal_id_1".into(),
                PeerId::random(),
                unit_ids,
            ))
            .await;

        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "Worker for deal_id_1 already exists"
        );
        // tokio doesn't allow to drop runtimes in async context, so shifting workers drop to the blocking thread
        tokio::task::spawn_blocking(|| drop(workers)).await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_remove() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();
        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        create_dirs(&[&workers_dir]).unwrap();

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);
        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor,
            thread_pinner,
        )
        .await
        .expect("Failed to create Workers from path");

        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let unit_ids = vec![init_id_1];

        let worker_id_1 = workers
            .create_worker(WorkerParams::new(
                "deal_id_1".into(),
                PeerId::random(),
                unit_ids.clone(),
            ))
            .await
            .expect("Failed to create worker");

        let worker_id_2 = workers
            .create_worker(WorkerParams::new(
                "deal_id_2".into(),
                PeerId::random(),
                unit_ids,
            ))
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
        let key_1 = key_storage.get_worker_key_pair(worker_id_1);
        let key_2 = key_storage.get_worker_key_pair(worker_id_2);
        assert!(key_1.is_some());
        assert!(key_2.is_none());
        // tokio doesn't allow to drop runtimes in async context, so shifting workers drop to the blocking thread
        tokio::task::spawn_blocking(|| drop(workers)).await.unwrap();
    }

    #[tokio::test]
    async fn test_persistence() {
        // Create a temporary directory for worker storage
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let key_pairs_dir = temp_dir.path().join("key_pairs").to_path_buf();
        let workers_dir = temp_dir.path().join("workers").to_path_buf();
        let root_key_pair = fluence_keypair::KeyPair::generate_ed25519();
        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        create_dirs(&[&workers_dir]).unwrap();

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);
        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );
        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor.clone(),
            thread_pinner,
        )
        .await
        .expect("Failed to create Workers from path");
        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let unit_ids = vec![init_id_1];

        let worker_id_1 = workers
            .create_worker(WorkerParams::new(
                "deal_id_1".into(),
                PeerId::random(),
                unit_ids.clone(),
            ))
            .await
            .expect("Failed to create worker");

        let worker_id_2 = workers
            .create_worker(WorkerParams::new(
                "deal_id_2".into(),
                PeerId::random(),
                unit_ids,
            ))
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
        let key_1 = key_storage.get_worker_key_pair(worker_id_1);
        let key_2 = key_storage.get_worker_key_pair(worker_id_2);
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
        // tokio doesn't allow to drop runtimes in async context, so shifting workers drop to the blocking thread
        tokio::task::spawn_blocking(|| drop(workers)).await.unwrap();

        // Create a new KeyStorage instance
        let key_storage = Arc::new(
            KeyStorage::from_path(key_pairs_dir.clone(), root_key_pair.clone())
                .await
                .expect("Failed to create KeyStorage from path"),
        );

        let core_distributor = DummyCoreDistibutor::new();
        let core_distributor = Arc::new(core_distributor);

        let thread_pinner = Arc::new(test_utils::pinning::DUMMY);
        let config = WorkersConfig::new(32, None);

        // Create a new Workers instance
        let (workers, _receiver) = Workers::from_path(
            config,
            workers_dir.clone(),
            key_storage.clone(),
            core_distributor,
            thread_pinner,
        )
        .await
        .expect("Failed to create Workers from path");

        let list = workers.list_workers();
        let expected_list = vec![worker_id_1];

        assert_eq!(list, expected_list);
        let key_1 = key_storage.get_worker_key_pair(worker_id_1);
        let key_2 = key_storage.get_worker_key_pair(worker_id_2);
        assert!(key_1.is_some());
        assert!(key_2.is_none());
        let status = workers.is_worker_active(worker_id_1);
        assert!(!status);
        // tokio doesn't allow to drop runtimes in async context, so shifting workers drop to the blocking thread
        tokio::task::spawn_blocking(|| drop(workers)).await.unwrap();
    }
}
