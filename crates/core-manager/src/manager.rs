use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::hash::{BuildHasherDefault, Hash};
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::sync::Arc;

use async_trait::async_trait;
use core_affinity::{set_mask_for_current, CoreId};
use enum_dispatch::enum_dispatch;
use fxhash::{FxBuildHasher, FxHasher};
use hwloc2::{ObjectType, Topology, TypeDepthError};
use parking_lot::RwLock;
use range_set_blaze::RangeSetBlaze;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use types::unit_id::UnitId;

use crate::core_range::CoreRange;

extern crate hwloc2;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalCoreId(#[serde_as(as = "DisplayFromStr")] usize);

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogicalCoreId(#[serde_as(as = "DisplayFromStr")] pub usize);

#[enum_dispatch]
pub trait CoreManagerFunctions {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AssignError>;

    fn release(&self, unit_ids: Vec<UnitId>);

    fn system_cpu_assignment(&self) -> Assignment;

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
    pub fn new(
        file_name: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
    ) -> Result<(Self, PersistenceTask), Error> {
        let available_core_count = core_range.0.len() as usize;

        if system_cpu_count == 0 {
            return Err(Error::IllegalSystemCoreCount);
        }

        if system_cpu_count > available_core_count {
            return Err(Error::NotEnoughCores {
                available: available_core_count,
                required: system_cpu_count,
            });
        }

        let topology = Topology::new().ok_or(Error::CreateTopology)?;

        let cores = topology
            .objects_with_type(&ObjectType::Core)
            .map_err(|err| Error::CollectCoresData { err })?;

        let mut cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId> =
            MultiMap::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let mut available_cores: BTreeSet<PhysicalCoreId> = BTreeSet::new();

        for physical_core in cores {
            let physical_core_id = physical_core.os_index() as usize;
            if core_range.0.contains(physical_core_id) {
                let logical_cores = physical_core.children();
                available_cores.insert(PhysicalCoreId(physical_core_id));
                for logical_core in logical_cores {
                    let logical_core_id = logical_core.os_index() as usize;
                    cores_mapping.insert(
                        PhysicalCoreId(physical_core_id),
                        LogicalCoreId(logical_core_id),
                    )
                }
            }
        }

        let mut system_cores: BTreeSet<PhysicalCoreId> = BTreeSet::new();
        for _ in 0..system_cpu_count {
            system_cores.insert(available_cores.pop_first().expect("Can't be empty"));
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

    pub fn from_path(
        file_name: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
    ) -> Result<(Self, PersistenceTask), LoadingError> {
        let exists = file_name.exists();
        if exists {
            let bytes = std::fs::read(&file_name).map_err(|err| LoadingError::IoError { err })?;
            let toml = std::str::from_utf8(bytes.as_slice())
                .map_err(|err| LoadingError::DecodeError { err })?;
            let persistent_state: CoreManagerState =
                toml::from_str(toml).map_err(|err| LoadingError::DeserializationError { err })?;

            let config_range = core_range.clone().0;
            let mut loaded_range = RangeSetBlaze::new();
            for core_id in persistent_state.cores_mapping.keys() {
                loaded_range.insert(core_id.0);
            }

            if config_range == loaded_range
                && persistent_state.system_cores.len() == system_cpu_count
            {
                Ok(Self::make_instance_with_task(file_name, persistent_state))
            } else {
                tracing::warn!(target: "core-manager", "The initial config has been changed. Ignoring the previous state");
                let (core_manager, task) =
                    Self::new(file_name.clone(), system_cpu_count, core_range)
                        .map_err(|err| LoadingError::CreateCoreManager { err })?;
                core_manager
                    .persist()
                    .map_err(|err| LoadingError::PersistError { err })?;
                Ok((core_manager, task))
            }
        } else {
            tracing::debug!(target: "core-manager", "The previous state was not found. Skipping...");
            let (core_manager, task) = Self::new(file_name.clone(), system_cpu_count, core_range)
                .map_err(|err| LoadingError::CreateCoreManager { err })?;
            core_manager
                .persist()
                .map_err(|err| LoadingError::PersistError { err })?;
            Ok((core_manager, task))
        }
    }
}

pub struct PersistenceTask {
    receiver: Receiver<()>,
}

impl PersistenceTask {
    async fn process_events(core_manager: Arc<CoreManager>, mut receiver: Receiver<()>) {
        let core_manager = core_manager.clone();
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
        .expect("Could not join persist task")
    }

    async fn task(self, core_manager: Arc<CoreManager>) {
        let persist_task = Self::process_events(core_manager, self.receiver);
        tokio::pin!(persist_task);
        loop {
            tokio::select! {
            _ = &mut persist_task => {
                }
            }
        }
    }
    pub async fn run(self, core_manager: Arc<CoreManager>) {
        tokio::task::Builder::new()
            .name("core-manager-persist")
            .spawn(self.task(core_manager))
            .expect("Could not spawn persist task");
    }
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum WorkType {
    CapacityCommitment,
    Deal,
}

pub struct AcquireRequest {
    unit_ids: Vec<UnitId>,
    worker_type: WorkType,
}

impl AcquireRequest {
    pub fn new(unit_ids: Vec<UnitId>, worker_type: WorkType) -> Self {
        Self {
            unit_ids,
            worker_type,
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("System core count should be > 0")]
    IllegalSystemCoreCount,
    #[error("Too much system cores needed. Required: {required}, available: {required}")]
    NotEnoughCores { available: usize, required: usize },
    #[error("Failed to create topology")]
    CreateTopology,
    #[error("Failed to collect cores data {err:?}")]
    CollectCoresData { err: TypeDepthError },
}

#[derive(Debug, Error)]
pub enum LoadingError {
    #[error(transparent)]
    CreateCoreManager {
        #[from]
        err: Error,
    },
    #[error(transparent)]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error(transparent)]
    DecodeError {
        #[from]
        err: Utf8Error,
    },
    #[error(transparent)]
    DeserializationError {
        #[from]
        err: toml::de::Error,
    },
    #[error(transparent)]
    PersistError {
        #[from]
        err: PersistError,
    },
}

#[derive(Debug, Error)]
pub enum PersistError {
    #[error(transparent)]
    IoError {
        #[from]
        err: std::io::Error,
    },
    #[error(transparent)]
    SerializationError {
        #[from]
        err: toml::ser::Error,
    },
}

#[derive(Debug, Error)]
pub enum AssignError {
    #[error("Not found free cores")]
    NotFoundAvailableCores,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Assignment {
    pub physical_core_ids: BTreeSet<PhysicalCoreId>,
    pub logical_core_ids: BTreeSet<LogicalCoreId>,
}

impl Assignment {
    pub fn pin_current_thread(&self) {
        let cores: Vec<CoreId> = self
            .logical_core_ids
            .iter()
            .map(|core_id| CoreId { id: core_id.0 })
            .collect();
        set_mask_for_current(&cores);
    }
}

impl CoreManagerFunctions for PersistentCoreManager {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AssignError> {
        let mut lock = self.state.write();
        let mut physical_core_ids = BTreeSet::new();
        let mut logical_core_ids = BTreeSet::new();
        let worker_unit_type = assign_request.worker_type;
        for unit_id in assign_request.unit_ids {
            let core_id = lock.unit_id_mapping.get_by_right(&unit_id).cloned();
            let core_id = match core_id {
                None => {
                    let core_id = lock
                        .available_cores
                        .pop_last()
                        .ok_or(AssignError::NotFoundAvailableCores)?;
                    lock.unit_id_mapping
                        .insert(core_id.clone(), unit_id.clone());
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
            physical_core_ids.insert(core_id.clone());
            let local_physical_core_ids = lock
                .cores_mapping
                .get_vec(&core_id)
                .expect("Can't be empty");
            for local_physical_core_id in local_physical_core_ids {
                let local_physical_core_id = local_physical_core_id.clone();
                logical_core_ids.insert(local_physical_core_id);
            }
        }

        // We are trying to notify a persistence task that the state has been changed.
        // We don't care if the channel is full, it means the current state will be stored with the previous event
        let _ = self.sender.try_send(());

        Ok(Assignment {
            physical_core_ids,
            logical_core_ids,
        })
    }

    fn release(&self, unit_ids: Vec<UnitId>) {
        let mut lock = self.state.write();
        for unit_id in unit_ids {
            if let Some(core_id) = lock.unit_id_mapping.get_by_right(&unit_id).cloned() {
                lock.available_cores.insert(core_id.clone());
                lock.unit_id_mapping.remove_by_left(&core_id);
                lock.work_type_mapping.remove(&unit_id);
            }
        }
    }

    fn system_cpu_assignment(&self) -> Assignment {
        let lock = self.state.read();
        let mut logical_core_ids = BTreeSet::new();
        for core in &lock.system_cores {
            let core_ids = lock.cores_mapping.get_vec(core).cloned().unwrap();
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
        let toml = toml::to_string_pretty(&inner_state)
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
        let physical_core_ids = (0..num_cpus::get_physical()).map(PhysicalCoreId).collect();
        let logical_core_ids = (0..num_cpus::get()).map(LogicalCoreId).collect();
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
    ) -> Result<Assignment, AssignError> {
        Ok(self.all_cores())
    }

    fn release(&self, _unit_ids: Vec<UnitId>) {}

    fn system_cpu_assignment(&self) -> Assignment {
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

            let (manager, _task) = PersistentCoreManager::new(
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
            let (manager, _task) = PersistentCoreManager::new(
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
