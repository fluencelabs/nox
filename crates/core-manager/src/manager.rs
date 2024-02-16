use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::hash::{BuildHasherDefault, Hash};
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::Utf8Error;
use std::sync::Arc;

use async_trait::async_trait;
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

const FILE_NAME: &str = "cores_state.toml";

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalCoreId(#[serde_as(as = "DisplayFromStr")] usize);

#[serde_as]
#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LogicalCoreId(#[serde_as(as = "DisplayFromStr")] pub usize);

#[enum_dispatch]
pub trait CoreManagerFunctions {
    fn assign_worker_core(&self, assign_request: AssignRequest) -> Result<Assignment, AssignError>;

    fn free(&self, unit_ids: Vec<UnitId>);

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
    inner: RwLock<InnerState>,
    sender: tokio::sync::mpsc::Sender<()>,
}

impl PersistentCoreManager {
    pub fn new(
        file_name: PathBuf,
        system_cpu_count: usize,
        cpus_range: Option<CoreRange>,
    ) -> Result<(Self, PersistenceTask), Error> {
        let core_range = Self::get_range(cpus_range);

        let available_core_count = core_range.len() as usize;

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

        let mut available_cores: Vec<PhysicalCoreId> = Vec::with_capacity(available_core_count);

        for physical_core in cores {
            let physical_core_id = physical_core.os_index() as usize;
            if core_range.contains(physical_core_id) {
                let logical_cores = physical_core.children();
                for logical_core in logical_cores {
                    let logical_core_id = logical_core.os_index() as usize;
                    available_cores.push(PhysicalCoreId(physical_core_id));
                    cores_mapping.insert(
                        PhysicalCoreId(physical_core_id),
                        LogicalCoreId(logical_core_id),
                    )
                }
            }
        }

        let system_cores: BTreeSet<PhysicalCoreId> =
            available_cores.drain(..system_cpu_count).collect();

        let unit_id_mapping = BiMap::with_capacity_and_hashers(
            available_core_count,
            FxBuildHasher::default(),
            FxBuildHasher::default(),
        );

        let type_mapping =
            Map::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let inner_state = InnerState {
            cores_mapping,
            system_cores,
            available_cores,
            unit_id_mapping,
            type_mapping,
        };

        let result = Self::make_instance_with_task(file_name, inner_state);

        Ok(result)
    }

    fn make_instance_with_task(
        file_name: PathBuf,
        inner_state: InnerState,
    ) -> (Self, PersistenceTask) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        (
            Self {
                file_path: file_name,
                sender,
                inner: RwLock::new(inner_state),
            },
            PersistenceTask { receiver },
        )
    }

    pub fn from_path(
        base_dir: PathBuf,
        system_cpu_count: usize,
        cpus_range: Option<CoreRange>,
    ) -> Result<(Self, PersistenceTask), LoadingError> {
        let state_file = base_dir.join(FILE_NAME);
        let exists = state_file.exists();
        if exists {
            let bytes = std::fs::read(&state_file).map_err(|err| LoadingError::IoError { err })?;
            let toml = std::str::from_utf8(bytes.as_slice())
                .map_err(|err| LoadingError::DecodeError { err })?;
            let persistent_state: InnerState =
                toml::from_str(toml).map_err(|err| LoadingError::DeserializationError { err })?;

            let config_range = Self::get_range(cpus_range.clone());
            let mut loaded_range = RangeSetBlaze::new();
            for core_id in persistent_state.cores_mapping.keys() {
                loaded_range.insert(core_id.0);
            }

            if config_range == loaded_range
                && persistent_state.system_cores.len() == system_cpu_count
            {
                Ok(Self::make_instance_with_task(state_file, persistent_state))
            } else {
                tracing::warn!(target: "core-manager", "The initial config has been changed. Ignoring the previous state");
                let (core_manager, task) =
                    Self::new(state_file.clone(), system_cpu_count, cpus_range)
                        .map_err(|err| LoadingError::CreateCoreManager { err })?;
                core_manager
                    .persist()
                    .map_err(|err| LoadingError::PersistError { err })?;
                Ok((core_manager, task))
            }
        } else {
            tracing::debug!(target: "core-manager", "The previous state was not found. Skipping...");
            let (core_manager, task) = Self::new(state_file.clone(), system_cpu_count, cpus_range)
                .map_err(|err| LoadingError::CreateCoreManager { err })?;
            core_manager
                .persist()
                .map_err(|err| LoadingError::PersistError { err })?;
            Ok((core_manager, task))
        }
    }

    fn get_range(cpus_range: Option<CoreRange>) -> RangeSetBlaze<usize> {
        cpus_range
            .map(|range| range.0)
            .unwrap_or(RangeSetBlaze::from_iter(0..num_cpus::get_physical()))
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
struct InnerState {
    // mapping between physical and logical cores
    cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId>,
    // allocated system cores
    system_cores: BTreeSet<PhysicalCoreId>,
    // free physical cores
    available_cores: Vec<PhysicalCoreId>,
    // mapping between physical core id and unit id
    unit_id_mapping: BiMap<PhysicalCoreId, UnitId>,
    // mapping between unit id and workload type
    type_mapping: Map<UnitId, WorkerType>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub enum WorkerType {
    CC,
    Worker,
}

pub struct AssignRequest {
    unit_ids: Vec<UnitId>,
    worker_type: WorkerType,
}

impl AssignRequest {
    pub fn new(unit_ids: Vec<UnitId>, worker_type: WorkerType) -> Self {
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

impl CoreManagerFunctions for PersistentCoreManager {
    fn assign_worker_core(&self, assign_request: AssignRequest) -> Result<Assignment, AssignError> {
        let mut lock = self.inner.write();
        let mut physical_core_ids = BTreeSet::new();
        let mut logical_core_ids = BTreeSet::new();
        let worker_unit_type = assign_request.worker_type;
        for unit_id in assign_request.unit_ids {
            let core_id = lock.unit_id_mapping.get_by_right(&unit_id).cloned();
            let core_id = match core_id {
                None => {
                    let core_id = lock
                        .available_cores
                        .pop()
                        .ok_or(AssignError::NotFoundAvailableCores)?;
                    lock.unit_id_mapping
                        .insert(core_id.clone(), unit_id.clone());
                    lock.type_mapping.insert(unit_id, worker_unit_type.clone());
                    core_id
                }
                Some(core_id) => {
                    lock.type_mapping.insert(unit_id, worker_unit_type.clone());
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

        let _ = self.sender.try_send(());

        Ok(Assignment {
            physical_core_ids,
            logical_core_ids,
        })
    }

    fn free(&self, unit_ids: Vec<UnitId>) {
        let mut lock = self.inner.write();
        for unit_id in unit_ids {
            if let Some(core_id) = lock.unit_id_mapping.get_by_right(&unit_id).cloned() {
                lock.available_cores.push(core_id.clone());
                lock.unit_id_mapping.remove_by_left(&core_id);
                lock.type_mapping.remove(&unit_id);
            }
        }
    }

    fn system_cpu_assignment(&self) -> Assignment {
        let lock = self.inner.read();
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
        let lock = self.inner.read();
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
    fn assign_worker_core(
        &self,
        _assign_request: AssignRequest,
    ) -> Result<Assignment, AssignError> {
        Ok(self.all_cores())
    }

    fn free(&self, _unit_ids: Vec<UnitId>) {}

    fn system_cpu_assignment(&self) -> Assignment {
        self.all_cores()
    }
    fn persist(&self) -> Result<(), PersistError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::manager::{AssignRequest, CoreManagerFunctions, PersistentCoreManager, WorkerType};

    #[test]
    fn test_assignment_and_switching() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let (manager, _task) =
            PersistentCoreManager::new(temp_dir.path().join("test.toml"), 2, None).unwrap();
        let unit_ids = vec!["1".into(), "2".into()];
        let assignment_1 = manager
            .assign_worker_core(AssignRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkerType::CC,
            })
            .unwrap();
        let assignment_2 = manager
            .assign_worker_core(AssignRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkerType::Worker,
            })
            .unwrap();
        let assignment_3 = manager
            .assign_worker_core(AssignRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkerType::CC,
            })
            .unwrap();
        assert_eq!(assignment_1, assignment_2);
        assert_eq!(assignment_1, assignment_3);
    }
}
