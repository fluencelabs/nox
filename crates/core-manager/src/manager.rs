use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::hash::BuildHasherDefault;
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use cpu_utils::{CPUTopology, LogicalCoreId, PhysicalCoreId};
use enum_dispatch::enum_dispatch;
use fxhash::{FxBuildHasher, FxHasher};
use parking_lot::RwLock;
use range_set_blaze::RangeSetBlaze;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;
use types::unit_id::UnitId;

use crate::core_range::CoreRange;
use crate::errors::{AcquireError, CreateError, LoadingError, PersistError};
use crate::types::{AcquireRequest, Assignment, WorkType};

type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

/// The `CoreManagerFunctions` trait defines operations for managing CPU cores.
///
/// Implement this trait to enable core acquisition, release, retrieval of system CPU assignments,
/// and persistence of the core manager's state.
///
/// # Trait Functions:
///
/// - `acquire_worker_core(assign_request: AcquireRequest) -> Result<Assignment, AcquireError>`:
///   Acquires CPU cores for a set of unit IDs and a specified worker type.
///
/// - `release(unit_ids: Vec<UnitId>)`:
///   Releases previously acquired CPU cores associated with a set of unit IDs.
///
/// - `get_system_cpu_assignment() -> Assignment`:
///   Retrieves the system's CPU assignment, including physical and logical core IDs.
///
/// - `persist() -> Result<(), PersistError>`:
///   Persists the current state of the core manager to an external storage location.
///
/// # Implementing Types:
///
/// - [`PersistentCoreManager`](struct.PersistentCoreManager.html):
///   Manages CPU cores persistently by saving and loading state to/from a file path.
///
/// - [`DummyCoreManager`](struct.DummyCoreManager.html):
///   Provides a dummy implementation for non-persistent core management scenarios.
///
/// - [`CoreManager`](enum.CoreManager.html):
///   Enumerates persistent and dummy core managers, allowing flexible core management choices.
///
/// # Example Usage:
///
/// ```rust
/// use core_manager::{CoreManager, AcquireRequest, WorkType};
///
/// let (core_manager, persistence_task) = PersistentCoreManager::from_path("core_state.toml".into(), 2, CoreRange::default()).expect("Failed to create manager");
/// let unit_ids = vec!["1".into(), "2".into()];
///
/// // Acquire and release cores
/// let assignment = core_manager.acquire_worker_core(AcquireRequest { unit_ids, worker_type: WorkType::CapacityCommitment }).unwrap();
///
/// // Retrieve system CPU assignment
/// let system_assignment = core_manager.get_system_cpu_assignment();
///
/// // Run persistence task in the background
/// tokio::spawn(persistence_task.run(core_manager.clone()));
///
/// сore_manager.release(unit_ids);
/// ```
#[enum_dispatch]
pub trait CoreManagerFunctions {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError>;

    fn release(&self, unit_ids: Vec<UnitId>);

    fn get_system_cpu_assignment(&self) -> Assignment;

    fn persist(&self) -> Result<(), PersistError>;
}

#[enum_dispatch(CoreManagerFunctions)]
pub enum CoreManager {
    Persistent(PersistentCoreManager),
    Dummy(DummyCoreManager),
}

pub struct PersistentCoreManager {
    file_path: PathBuf,
    state: RwLock<CoreManagerState>,
    sender: tokio::sync::mpsc::Sender<()>,
}

impl PersistentCoreManager {
    /// Loads the state from `file_name` if exists. If not creates a new empty state
    pub fn from_path(
        file_path: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
    ) -> Result<(Self, PersistenceTask), LoadingError> {
        let exists = file_path.exists();
        if exists {
            let bytes = std::fs::read(&file_path).map_err(|err| LoadingError::IoError { err })?;
            let raw_str = std::str::from_utf8(bytes.as_slice())
                .map_err(|err| LoadingError::DecodeError { err })?;
            let persistent_state: PersistentCoreManagerState = toml::from_str(raw_str)
                .map_err(|err| LoadingError::DeserializationError { err })?;

            let config_range = core_range.clone().0;
            let mut loaded_range = RangeSetBlaze::new();
            for (physical_core_id, _) in persistent_state.cores_mapping.clone() {
                loaded_range.insert(<PhysicalCoreId as Into<u32>>::into(physical_core_id) as usize);
            }

            if config_range == loaded_range && persistent_state.system_cores.len() == system_cpu_count {
                let state: CoreManagerState = persistent_state.into();
                Ok(Self::make_instance_with_task(file_path, state))
            } else {
                tracing::warn!(target: "core-manager", "The initial config has been changed. Ignoring the previous state");
                let (core_manager, task) =
                    Self::new(file_path.clone(), system_cpu_count, core_range)
                        .map_err(|err| LoadingError::CreateCoreManager { err })?;
                core_manager
                    .persist()
                    .map_err(|err| LoadingError::PersistError { err })?;
                Ok((core_manager, task))
            }
        } else {
            tracing::debug!(target: "core-manager", "The previous state was not found. Creating a new one.");
            let (core_manager, task) = Self::new(file_path.clone(), system_cpu_count, core_range)
                .map_err(|err| LoadingError::CreateCoreManager { err })?;
            core_manager
                .persist()
                .map_err(|err| LoadingError::PersistError { err })?;
            Ok((core_manager, task))
        }
    }

    /// Creates an empty core manager with only system cores assigned
    fn new(
        file_name: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
    ) -> Result<(Self, PersistenceTask), CreateError> {
        let available_core_count = core_range.0.len() as usize;

        if system_cpu_count == 0 {
            return Err(CreateError::IllegalSystemCoreCount);
        }

        if system_cpu_count > available_core_count {
            return Err(CreateError::NotEnoughCores {
                available: available_core_count,
                required: system_cpu_count,
            });
        }

        // to observe CPU topology
        let topology = CPUTopology::new().map_err(|err| CreateError::CreateTopology { err })?;

        // retrieve info about physical cores
        let physical_cores = topology
            .physical_cores()
            .map_err(|err| CreateError::CollectCoresData { err })?;

        let mut cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId> =
            MultiMap::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let mut available_cores: BTreeSet<PhysicalCoreId> = BTreeSet::new();

        for physical_core_id in physical_cores {
            if core_range
                .0
                .contains(<PhysicalCoreId as Into<u32>>::into(physical_core_id) as usize as usize)
            {
                let logical_cores = topology
                    .logical_cores_for_physical(physical_core_id)
                    .map_err(|err| CreateError::CollectCoresData { err })?;
                available_cores.insert(physical_core_id);
                for logical_core_id in logical_cores {
                    cores_mapping.insert(physical_core_id, logical_core_id)
                }
            }
        }

        let mut system_cores: BTreeSet<PhysicalCoreId> = BTreeSet::new();
        for _ in 0..system_cpu_count {
            system_cores.insert(
                available_cores
                    .pop_first()
                    .expect("Unexpected state. Should not be empty never"),
            );
        }

        let unit_id_mapping = BiMap::with_capacity_and_hashers(
            available_core_count,
            FxBuildHasher::default(),
            FxBuildHasher::default(),
        );

        let type_mapping =
            Map::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let inner_state = CoreManagerState {
            cores_mapping,
            system_cores,
            available_cores,
            unit_id_mapping,
            work_type_mapping: type_mapping,
        };

        let result = Self::make_instance_with_task(file_name, inner_state);

        Ok(result)
    }

    fn make_instance_with_task(
        file_name: PathBuf,
        state: CoreManagerState,
    ) -> (Self, PersistenceTask) {
        // This channel is used to notify a persistent task about changes.
        // It has a size of 1 because we need only the fact that this change happen
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                file_path: file_name,
                sender,
                state: RwLock::new(state),
            },
            PersistenceTask { receiver },
        )
    }
}

pub struct PersistenceTask {
    receiver: Receiver<()>,
}

impl PersistenceTask {
    async fn process_events(core_manager: Arc<CoreManager>, mut receiver: Receiver<()>) {
        let core_manager = core_manager.clone();
        // We are not interested in the content of the event
        // We are waiting for the event to initiate the persistence process
        let _ = receiver.recv().await;
        tokio::task::spawn_blocking(move || {
            let result = core_manager.persist();
            match result {
                Ok(_) => {
                    tracing::debug!(target: "core-manager", "Core state was persisted");
                }
                Err(err) => {
                    tracing::warn!(target: "core-manager", "Failed to save core state {err}");
                }
            }
        })
        .await
        .expect("Could not spawn persist task")
    }

    async fn persistence_task(self, core_manager: Arc<CoreManager>) {
        let persist_task = Self::process_events(core_manager, self.receiver);
        tokio::pin!(persist_task);
        loop {
            tokio::select! {
            _ = &mut persist_task => {}
            }
        }
    }
    pub async fn run(self, core_manager: Arc<CoreManager>) {
        tokio::task::Builder::new()
            .name("core-manager-persist")
            .spawn(self.persistence_task(core_manager))
            .expect("Could not spawn persist task");
    }
}

struct CoreManagerState {
    // mapping between physical and logical cores
    cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId>,
    // allocated system cores
    system_cores: BTreeSet<PhysicalCoreId>,
    // free physical cores
    available_cores: BTreeSet<PhysicalCoreId>,
    // mapping between physical core id and unit id
    unit_id_mapping: BiMap<PhysicalCoreId, UnitId>,
    // mapping between unit id and workload type
    work_type_mapping: Map<UnitId, WorkType>,
}

#[derive(Serialize, Deserialize)]
struct PersistentCoreManagerState {
    cores_mapping: Vec<(PhysicalCoreId, LogicalCoreId)>,
    system_cores: Vec<PhysicalCoreId>,
    available_cores: Vec<PhysicalCoreId>,
    unit_id_mapping: Vec<(PhysicalCoreId, UnitId)>,
    work_type_mapping: Vec<(UnitId, WorkType)>,
}

impl From<&CoreManagerState> for PersistentCoreManagerState {
    fn from(value: &CoreManagerState) -> Self {
        Self {
            cores_mapping: value.cores_mapping.iter().map(|(k, v)| (*k, *v)).collect(),
            system_cores: value.system_cores.iter().cloned().collect(),
            available_cores: value.available_cores.iter().cloned().collect(),
            unit_id_mapping: value
                .unit_id_mapping
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            work_type_mapping: value
                .work_type_mapping
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

impl From<PersistentCoreManagerState> for CoreManagerState {
    fn from(value: PersistentCoreManagerState) -> Self {
        Self {
            cores_mapping: value.cores_mapping.into_iter().collect(),
            system_cores: value.system_cores.into_iter().collect(),
            available_cores: value.available_cores.into_iter().collect(),
            unit_id_mapping: value.unit_id_mapping.into_iter().collect(),
            work_type_mapping: value.work_type_mapping.into_iter().collect(),
        }
    }
}

impl CoreManagerFunctions for PersistentCoreManager {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut lock = self.state.write();
        let mut result_physical_core_ids = BTreeSet::new();
        let mut result_logical_core_ids = BTreeSet::new();
        let worker_unit_type = assign_request.worker_type;
        for unit_id in assign_request.unit_ids {
            let physical_core_id = lock.unit_id_mapping.get_by_right(&unit_id).cloned();
            let physical_core_id = match physical_core_id {
                None => {
                    let core_id = lock
                        .available_cores
                        .pop_last()
                        .ok_or(AcquireError::NotFoundAvailableCores)?;
                    lock.unit_id_mapping.insert(core_id, unit_id.clone());
                    lock.work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    core_id
                }
                Some(core_id) => {
                    lock.work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    core_id
                }
            };
            result_physical_core_ids.insert(physical_core_id);

            let physical_core_ids = lock
                .cores_mapping
                .get_vec(&physical_core_id)
                .cloned()
                .expect("Unexpected state. Should not be empty never");

            for physical_core_id in physical_core_ids {
                result_logical_core_ids.insert(physical_core_id);
            }
        }

        // We are trying to notify a persistence task that the state has been changed.
        // We don't care if the channel is full, it means the current state will be stored with the previous event
        let _ = self.sender.try_send(());

        Ok(Assignment {
            physical_core_ids: result_physical_core_ids,
            logical_core_ids: result_logical_core_ids,
        })
    }

    fn release(&self, unit_ids: Vec<UnitId>) {
        let mut lock = self.state.write();
        for unit_id in unit_ids {
            if let Some((physical_core_id, _)) = lock.unit_id_mapping.remove_by_right(&unit_id) {
                lock.available_cores.insert(physical_core_id);
                lock.work_type_mapping.remove(&unit_id);
            }
        }
    }

    fn get_system_cpu_assignment(&self) -> Assignment {
        let lock = self.state.read();
        let mut logical_core_ids = BTreeSet::new();
        for core in &lock.system_cores {
            let core_ids = lock
                .cores_mapping
                .get_vec(core)
                .cloned()
                .expect("Unexpected state. Should not be empty never");
            for core_id in core_ids {
                logical_core_ids.insert(core_id);
            }
        }
        Assignment {
            physical_core_ids: lock.system_cores.clone(),
            logical_core_ids,
        }
    }

    fn persist(&self) -> Result<(), PersistError> {
        let lock = self.state.read();
        let inner_state = lock.deref();
        let persistent_state: PersistentCoreManagerState = inner_state.into();
        drop(lock);
        let toml = toml::to_string_pretty(&persistent_state)
            .map_err(|err| PersistError::SerializationError { err })?;
        let exists = self.file_path.exists();
        let mut file = if exists {
            File::open(self.file_path.clone()).map_err(|err| PersistError::IoError { err })?
        } else {
            File::create(self.file_path.clone()).map_err(|err| PersistError::IoError { err })?
        };
        file.write(toml.as_bytes())
            .map_err(|err| PersistError::IoError { err })?;
        Ok(())
    }
}

#[derive(Default)]
pub struct DummyCoreManager {}

impl DummyCoreManager {
    fn all_cores(&self) -> Assignment {
        let physical_core_ids = (0..num_cpus::get_physical())
            .map(|v| PhysicalCoreId::from(v as u32))
            .collect();
        let logical_core_ids = (0..num_cpus::get())
            .map(|v| LogicalCoreId::from(v as u32))
            .collect();
        Assignment {
            physical_core_ids,
            logical_core_ids,
        }
    }
}

#[async_trait]
impl CoreManagerFunctions for DummyCoreManager {
    fn acquire_worker_core(
        &self,
        _assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        Ok(self.all_cores())
    }

    fn release(&self, _unit_ids: Vec<UnitId>) {}

    fn get_system_cpu_assignment(&self) -> Assignment {
        self.all_cores()
    }
    fn persist(&self) -> Result<(), PersistError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::manager::{AcquireRequest, CoreManagerFunctions, PersistentCoreManager, WorkType};
    use crate::CoreRange;

    fn cores_exists() -> bool {
        num_cpus::get_physical() >= 4
    }

    #[test]
    fn test_acquire_and_switch() {
        if cores_exists() {
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

            let (manager, _task) = PersistentCoreManager::from_path(
                temp_dir.path().join("test.toml"),
                2,
                CoreRange::default(),
            )
            .unwrap();
            let unit_ids = vec!["1".into(), "2".into()];
            let assignment_1 = manager
                .acquire_worker_core(AcquireRequest {
                    unit_ids: unit_ids.clone(),
                    worker_type: WorkType::CapacityCommitment,
                })
                .unwrap();
            let assignment_2 = manager
                .acquire_worker_core(AcquireRequest {
                    unit_ids: unit_ids.clone(),
                    worker_type: WorkType::Deal,
                })
                .unwrap();
            let assignment_3 = manager
                .acquire_worker_core(AcquireRequest {
                    unit_ids: unit_ids.clone(),
                    worker_type: WorkType::CapacityCommitment,
                })
                .unwrap();
            assert_eq!(assignment_1, assignment_2);
            assert_eq!(assignment_1, assignment_3);
        }
    }

    #[test]
    fn test_acquire_and_release() {
        if cores_exists() {
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let system_cpu_count = 2;
            let (manager, _task) = PersistentCoreManager::from_path(
                temp_dir.path().join("test.toml"),
                system_cpu_count,
                CoreRange::default(),
            )
            .unwrap();
            let before_lock = manager.state.read();

            let before_available_core = before_lock.available_cores.clone();
            let before_unit_id_mapping = before_lock.unit_id_mapping.clone();
            let before_type_mapping = before_lock.work_type_mapping.clone();
            drop(before_lock);

            assert_eq!(
                before_available_core.len(),
                num_cpus::get_physical() - system_cpu_count
            );
            assert_eq!(before_unit_id_mapping.len(), 0);
            assert_eq!(before_type_mapping.len(), 0);

            let unit_ids = vec!["1".into(), "2".into()];
            let assignment = manager
                .acquire_worker_core(AcquireRequest {
                    unit_ids: unit_ids.clone(),
                    worker_type: WorkType::CapacityCommitment,
                })
                .unwrap();
            assert_eq!(assignment.physical_core_ids.len(), 2);

            let after_assignment = manager.state.read();

            let after_assignment_available_core = after_assignment.available_cores.clone();
            let after_assignment_unit_id_mapping = after_assignment.unit_id_mapping.clone();
            let after_assignment_type_mapping = after_assignment.work_type_mapping.clone();
            drop(after_assignment);

            assert_eq!(
                after_assignment_available_core.len(),
                before_available_core.len() - 2
            );
            assert_eq!(after_assignment_unit_id_mapping.len(), 2);
            assert_eq!(after_assignment_type_mapping.len(), 2);

            manager.release(unit_ids);

            let after_release_lock = manager.state.read();

            let after_release_available_core = after_release_lock.available_cores.clone();
            let after_release_unit_id_mapping = after_release_lock.unit_id_mapping.clone();
            let after_release_type_mapping = after_release_lock.work_type_mapping.clone();
            drop(after_release_lock);

            assert_eq!(after_release_available_core, before_available_core);
            assert_eq!(after_release_unit_id_mapping, before_unit_id_mapping);
            assert_eq!(after_release_type_mapping, before_type_mapping);
        }
    }
}
