use std::collections::{BTreeSet, HashMap};
use std::hash::BuildHasherDefault;
use std::ops::Deref;
use std::path::PathBuf;

use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use cpu_utils::CPUTopology;
use fxhash::{FxBuildHasher, FxHasher};
use parking_lot::RwLock;
use range_set_blaze::RangeSetBlaze;

use crate::errors::{AcquireError, CreateError, CurrentAssignment, LoadingError, PersistError};
use crate::manager::CoreManagerFunctions;
use crate::persistence::{
    PersistenceTask, PersistentCoreManagerFunctions, PersistentCoreManagerState,
};
use crate::types::{AcquireRequest, Assignment, WorkType};
use crate::CoreRange;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
pub(crate) type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

/// `StrictCoreManager` is a CPU core manager responsible for allocating and releasing CPU cores
/// based on workload requirements. It maintains the state of core allocations, persists
/// the state to disk, and provides methods for acquiring and releasing cores.
pub struct StrictCoreManager {
    // path to the persistent state
    file_path: PathBuf,
    // inner state
    state: RwLock<CoreManagerState>,
    // persistent task notification channel
    sender: tokio::sync::mpsc::Sender<()>,
}

impl StrictCoreManager {
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

            if config_range == loaded_range
                && persistent_state.system_cores.len() == system_cpu_count
            {
                let state: CoreManagerState = persistent_state.into();
                Ok(Self::make_instance_with_task(file_path, state))
            } else {
                tracing::warn!(target: "core-manager", "The initial config has been changed. Ignoring persisted core mapping");
                let (core_manager, task) =
                    Self::new(file_path.clone(), system_cpu_count, core_range)
                        .map_err(|err| LoadingError::CreateCoreManager { err })?;
                core_manager
                    .persist()
                    .map_err(|err| LoadingError::PersistError { err })?;
                Ok((core_manager, task))
            }
        } else {
            tracing::debug!(target: "core-manager", "No persisted core mapping was not found. Creating a new one.");
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
                .contains(<PhysicalCoreId as Into<u32>>::into(physical_core_id) as usize)
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
            PersistenceTask::new(receiver),
        )
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
    unit_id_mapping: BiMap<PhysicalCoreId, CUID>,
    // mapping between unit id and workload type
    work_type_mapping: Map<CUID, WorkType>,
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
                .map(|(k, v)| (*k, (*v)))
                .collect(),
            work_type_mapping: value
                .work_type_mapping
                .iter()
                .map(|(k, v)| ((*k), v.clone()))
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

impl CoreManagerFunctions for StrictCoreManager {
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
                    let core_id = lock.available_cores.pop_last().ok_or({
                        let current_assignment: Vec<(PhysicalCoreId, CUID)> =
                            lock.unit_id_mapping.iter().map(|(k, v)| (*k, *v)).collect();
                        AcquireError::NotFoundAvailableCores {
                            current_assignment: CurrentAssignment::new(current_assignment),
                        }
                    })?;
                    lock.unit_id_mapping.insert(core_id, unit_id);
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

    fn release(&self, unit_ids: Vec<CUID>) {
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
}

impl PersistentCoreManagerFunctions for StrictCoreManager {
    fn persist(&self) -> Result<(), PersistError> {
        let lock = self.state.read();
        let inner_state = lock.deref();
        let persistent_state: PersistentCoreManagerState = inner_state.into();
        drop(lock);
        persistent_state.persist(self.file_path.as_path())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
    use hex::FromHex;

    use crate::manager::CoreManagerFunctions;
    use crate::persistence::PersistentCoreManagerState;
    use crate::strict::StrictCoreManager;
    use crate::types::{AcquireRequest, WorkType};
    use crate::CoreRange;

    fn cores_exists() -> bool {
        num_cpus::get_physical() >= 4
    }

    #[test]
    fn test_acquire_and_switch() {
        if cores_exists() {
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

            let (manager, _task) = StrictCoreManager::from_path(
                temp_dir.path().join("test.toml"),
                2,
                CoreRange::default(),
            )
            .unwrap();
            let init_id_1 = <CUID>::from_hex(
                "54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea",
            )
            .unwrap();
            let init_id_2 = <CUID>::from_hex(
                "1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0",
            )
            .unwrap();
            let unit_ids = vec![init_id_1, init_id_2];
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
            let (manager, _task) = StrictCoreManager::from_path(
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

            let init_id_1 = <CUID>::from_hex(
                "54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea",
            )
            .unwrap();
            let init_id_2 = <CUID>::from_hex(
                "1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0",
            )
            .unwrap();
            let unit_ids = vec![init_id_1, init_id_2];
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

    #[test]
    fn test_acquire_error_message() {
        if cores_exists() {
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let init_id_1 = <CUID>::from_hex(
                "54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea",
            )
            .unwrap();
            let init_id_2 = <CUID>::from_hex(
                "1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0",
            )
            .unwrap();
            let init_id_3 = <CUID>::from_hex(
                "271e0e06fdae1f0237055e78f5804416fd9ebb9ca5b52ae360d8124cde220dae",
            )
            .unwrap();
            let persistent_state = PersistentCoreManagerState {
                cores_mapping: vec![
                    (PhysicalCoreId::new(1), LogicalCoreId::new(1)),
                    (PhysicalCoreId::new(1), LogicalCoreId::new(2)),
                    (PhysicalCoreId::new(2), LogicalCoreId::new(3)),
                    (PhysicalCoreId::new(2), LogicalCoreId::new(4)),
                    (PhysicalCoreId::new(3), LogicalCoreId::new(5)),
                    (PhysicalCoreId::new(3), LogicalCoreId::new(6)),
                ],
                system_cores: vec![PhysicalCoreId::new(1)],
                available_cores: vec![PhysicalCoreId::new(2)],
                unit_id_mapping: vec![(PhysicalCoreId::new(3), init_id_1)],
                work_type_mapping: vec![(init_id_1, WorkType::Deal)],
            };
            let (manager, _task) = StrictCoreManager::make_instance_with_task(
                temp_dir.into_path(),
                persistent_state.into(),
            );

            manager
                .acquire_worker_core(AcquireRequest {
                    unit_ids: vec![init_id_2],
                    worker_type: WorkType::Deal,
                })
                .unwrap();

            let result = manager.acquire_worker_core(AcquireRequest {
                unit_ids: vec![init_id_3],
                worker_type: WorkType::Deal,
            });

            let expected = "Couldn't assign core: no free cores left. \
            Current assignment: [2 -> 1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0, \
            3 -> 54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea]".to_string();
            assert_eq!(expected, result.unwrap_err().to_string());
        }
    }
}
