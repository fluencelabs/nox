use std::collections::VecDeque;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use cpu_utils::CPUTopology;
use fxhash::FxBuildHasher;
use parking_lot::RwLock;
use range_set_blaze::RangeSetBlaze;

use crate::errors::{AcquireError, CreateError, LoadingError, PersistError};
use crate::persistence::{PersistenceTask, PersistentCoreDistributorState, StatePersistence};
use crate::strategy::{AcquireStrategy, AcquireStrategyInner, AcquireStrategyOperations};
use crate::types::{AcquireRequest, Assignment, SystemAssignment, WorkType};
use crate::{BiMap, CoreRange, Map, MultiMap};

#[cfg_attr(feature = "mockall", mockall::automock)]
pub trait CoreDistributor: Send + Sync {
    fn acquire_worker_cores(
        &self,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError>;

    fn release_worker_cores(&self, unit_ids: &[CUID]);

    fn get_system_cpu_assignment(&self) -> SystemAssignment;
}

/// `StrictCoreManager` is a CPU core manager responsible for allocating and releasing CPU cores
/// based on workload requirements. It maintains the state of core allocations, persists
/// the state to disk, and provides methods for acquiring and releasing cores.
pub struct PersistentCoreDistributor {
    file_path: PathBuf,
    // inner state
    state: RwLock<CoreDistributorState>,
    // persistent task notification channel
    sender: tokio::sync::mpsc::Sender<()>,

    acquire_strategy: AcquireStrategyInner,
}

impl From<PersistentCoreDistributorState> for CoreDistributorState {
    fn from(value: PersistentCoreDistributorState) -> Self {
        Self {
            cores_mapping: value.cores_mapping.into_iter().collect(),
            system_cores: value.system_cores.into_iter().collect(),
            available_cores: value.available_cores.into_iter().collect(),
            unit_id_mapping: value.unit_id_mapping.into_iter().collect(),
            work_type_mapping: value.work_type_mapping.into_iter().collect(),
        }
    }
}

impl PersistentCoreDistributor {
    /// Loads the state from `file_name` if exists. If not creates a new empty state
    pub fn from_path(
        file_path: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
        acquire_strategy: AcquireStrategy,
        cpu_topology: &dyn CPUTopology,
    ) -> Result<(Arc<Self>, PersistenceTask), LoadingError> {
        let exists = file_path.exists();
        if exists {
            let bytes = std::fs::read(&file_path).map_err(|err| LoadingError::IoError { err })?;
            let raw_str = std::str::from_utf8(bytes.as_slice())
                .map_err(|err| LoadingError::DecodeError { err })?;
            let persistent_state: PersistentCoreDistributorState = toml::from_str(raw_str)
                .map_err(|err| LoadingError::DeserializationError { err })?;

            let config_range = core_range.clone().0;
            let mut loaded_range = RangeSetBlaze::new();
            for (physical_core_id, _) in persistent_state.cores_mapping.clone() {
                loaded_range.insert(<PhysicalCoreId as Into<u32>>::into(physical_core_id) as usize);
            }

            if config_range == loaded_range
                && persistent_state.system_cores.len() == system_cpu_count
            {
                let state: CoreDistributorState = persistent_state.into();
                Ok(Self::make_instance_with_task(
                    file_path,
                    state,
                    acquire_strategy,
                ))
            } else {
                tracing::warn!(target: "core-distributor", "The initial config has been changed. Ignoring persisted core mapping");
                let (core_distributor, task) = Self::new(
                    file_path.clone(),
                    system_cpu_count,
                    core_range,
                    acquire_strategy,
                    cpu_topology,
                )
                .map_err(|err| LoadingError::CreateCoreDistributor { err })?;
                core_distributor
                    .persist()
                    .map_err(|err| LoadingError::PersistError { err })?;
                Ok((core_distributor, task))
            }
        } else {
            tracing::debug!(target: "core-distributor", "No persisted core mapping was not found. Creating a new one.");
            let (core_distributor, task) = Self::new(
                file_path.clone(),
                system_cpu_count,
                core_range,
                acquire_strategy,
                cpu_topology,
            )
            .map_err(|err| LoadingError::CreateCoreDistributor { err })?;
            core_distributor
                .persist()
                .map_err(|err| LoadingError::PersistError { err })?;
            Ok((core_distributor, task))
        }
    }

    /// Creates an empty core manager with only system cores assigned
    fn new(
        file_name: PathBuf,
        system_cpu_count: usize,
        core_range: CoreRange,
        acquire_strategy: AcquireStrategy,
        cpu_topology: &dyn CPUTopology,
    ) -> Result<(Arc<Self>, PersistenceTask), CreateError> {
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

        // retrieve info about physical cores
        let physical_cores = cpu_topology
            .physical_cores()
            .map_err(|err| CreateError::CollectCoresData { err })?;

        if !core_range.is_subset(&physical_cores) {
            return Err(CreateError::WrongCpuRange);
        }

        let mut cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId> =
            MultiMap::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let mut available_cores: VecDeque<PhysicalCoreId> = VecDeque::new();

        for physical_core_id in physical_cores {
            if core_range
                .0
                .contains(<PhysicalCoreId as Into<u32>>::into(physical_core_id) as usize)
            {
                let logical_cores = cpu_topology
                    .logical_cores_for_physical(physical_core_id)
                    .map_err(|err| CreateError::CollectCoresData { err })?;
                available_cores.push_back(physical_core_id);
                for logical_core_id in logical_cores {
                    cores_mapping.insert(physical_core_id, logical_core_id)
                }
            }
        }

        let mut system_cores: Vec<PhysicalCoreId> = Vec::new();
        for _ in 0..system_cpu_count {
            // SAFETY: this should never happen because we already checked the availability of cores
            system_cores.push(
                available_cores
                    .pop_front()
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

        let inner_state = CoreDistributorState {
            cores_mapping,
            system_cores,
            available_cores,
            unit_id_mapping,
            work_type_mapping: type_mapping,
        };

        let result = Self::make_instance_with_task(file_name, inner_state, acquire_strategy);

        Ok(result)
    }

    fn make_instance_with_task(
        file_path: PathBuf,
        state: CoreDistributorState,
        acquire_strategy: AcquireStrategy,
    ) -> (Arc<Self>, PersistenceTask) {
        // This channel is used to notify a persistent task about changes.
        // It has a size of 1 because we need only the fact that this change happen
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        let distributor = Self {
            file_path,
            sender,
            state: RwLock::new(state),
            acquire_strategy: acquire_strategy.into(),
        };

        let distributor = Arc::new(distributor);
        let task = PersistenceTask::new(distributor.clone(), receiver);

        (distributor, task)
    }
}

impl StatePersistence for PersistentCoreDistributor {
    fn persist(&self) -> Result<(), PersistError> {
        let lock = self.state.read();
        let inner_state = lock.deref();
        let persistent_state: PersistentCoreDistributorState = inner_state.into();
        drop(lock);
        persistent_state.persist(self.file_path.as_path())?;
        Ok(())
    }
}

impl CoreDistributor for PersistentCoreDistributor {
    fn acquire_worker_cores(
        &self,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut lock = self.state.write();

        let result = self.acquire_strategy.acquire(&mut lock, acquire_request)?;

        // We are trying to notify a persistence task that the state has been changed.
        // We don't care if the channel is full, it means the current state will be stored with the previous event
        let _ = self.sender.try_send(());

        Ok(result)
    }

    fn release_worker_cores(&self, unit_ids: &[CUID]) {
        let mut lock = self.state.write();
        for unit_id in unit_ids {
            if let Some((physical_core_id, _)) = lock.unit_id_mapping.remove_by_right(unit_id) {
                lock.available_cores.push_back(physical_core_id);
                lock.work_type_mapping.remove(unit_id);
            }
        }
    }

    fn get_system_cpu_assignment(&self) -> SystemAssignment {
        let lock = self.state.read();
        let mut logical_core_ids = Vec::new();
        for core in &lock.system_cores {
            // SAFETY: The physical core always has corresponding logical ids,
            // system cores can't have a wrong physical_core_id
            let core_ids = lock
                .cores_mapping
                .get_vec(core)
                .cloned()
                .expect("Unexpected state. Should not be empty never");
            for core_id in core_ids {
                logical_core_ids.push(core_id);
            }
        }
        SystemAssignment::new(lock.system_cores.clone(), logical_core_ids)
    }
}

pub(crate) struct CoreDistributorState {
    // mapping between physical and logical cores
    pub cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId>,
    // allocated system cores
    pub system_cores: Vec<PhysicalCoreId>,
    // free physical cores
    pub available_cores: VecDeque<PhysicalCoreId>,
    // mapping between physical core id and unit id
    pub unit_id_mapping: BiMap<PhysicalCoreId, CUID>,
    // mapping between unit id and workload type
    pub work_type_mapping: Map<CUID, WorkType>,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::str::FromStr;

    use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
    use cpu_utils::MockCPUTopology;
    use hex::FromHex;
    use mockall::predicate::eq;
    use nonempty::nonempty;
    use rand::Rng;

    use crate::errors::AcquireError;
    use crate::persistence::PersistentCoreDistributorState;
    use crate::types::{AcquireRequest, WorkType};
    use crate::{AcquireStrategy, CoreDistributor, CoreRange, PersistentCoreDistributor};

    fn mocked_topology() -> MockCPUTopology {
        let mut cpu_topology = MockCPUTopology::new();

        cpu_topology.expect_physical_cores().returning(|| {
            Ok(nonempty![
                PhysicalCoreId::new(0),
                PhysicalCoreId::new(1),
                PhysicalCoreId::new(2),
                PhysicalCoreId::new(3),
                PhysicalCoreId::new(4),
                PhysicalCoreId::new(5),
                PhysicalCoreId::new(6),
                PhysicalCoreId::new(7),
            ])
        });

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(0)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(0), LogicalCoreId::new(1)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(1)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(2), LogicalCoreId::new(3)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(2)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(4), LogicalCoreId::new(5)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(3)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(6), LogicalCoreId::new(7)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(4)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(8), LogicalCoreId::new(9)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(5)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(10), LogicalCoreId::new(11)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(6)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(12), LogicalCoreId::new(13)]));

        cpu_topology
            .expect_logical_cores_for_physical()
            .with(eq(PhysicalCoreId::new(7)))
            .returning(|_| Ok(nonempty![LogicalCoreId::new(14), LogicalCoreId::new(15)]));

        cpu_topology
    }

    #[test]
    fn test_acquire_and_switch() {
        let cpu_topology = mocked_topology();
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let (manager, _task) = PersistentCoreDistributor::from_path(
            temp_dir.path().join("test.toml"),
            2,
            CoreRange::from_str("0-7").unwrap(),
            AcquireStrategy::Strict,
            &cpu_topology,
        )
        .unwrap();
        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let init_id_2 =
            <CUID>::from_hex("1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0")
                .unwrap();
        let unit_ids = vec![init_id_1, init_id_2];
        let assignment_1 = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::CapacityCommitment,
            })
            .unwrap();
        let assignment_2 = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::Deal,
            })
            .unwrap();
        let assignment_3 = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::CapacityCommitment,
            })
            .unwrap();
        assert_eq!(assignment_1, assignment_2);
        assert_eq!(
            assignment_1
                .cuid_cores
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from_iter(unit_ids)
        );
        assert_eq!(assignment_1, assignment_2);
        assert_eq!(assignment_1, assignment_3);
    }

    #[test]
    fn test_acquire_and_release() {
        let cpu_topology = mocked_topology();

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let system_cpu_count = 2;
        let (manager, _task) = PersistentCoreDistributor::from_path(
            temp_dir.path().join("test.toml"),
            system_cpu_count,
            CoreRange::from_str("0-7").unwrap(),
            AcquireStrategy::Strict,
            &cpu_topology,
        )
        .unwrap();
        let before_lock = manager.state.read();

        let mut before_available_core = before_lock
            .available_cores
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        before_available_core.sort();
        let before_unit_id_mapping = before_lock.unit_id_mapping.clone();
        let before_type_mapping = before_lock.work_type_mapping.clone();
        drop(before_lock);

        assert_eq!(before_available_core.len(), 6);
        assert_eq!(before_unit_id_mapping.len(), 0);
        assert_eq!(before_type_mapping.len(), 0);

        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let init_id_2 =
            <CUID>::from_hex("1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0")
                .unwrap();
        let unit_ids = vec![init_id_1, init_id_2];
        let assignment = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::CapacityCommitment,
            })
            .unwrap();
        assert_eq!(assignment.logical_core_ids().len(), 4);
        assert_eq!(assignment.cuid_cores.len(), 2);

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

        manager.release_worker_cores(&unit_ids);

        let after_release_lock = manager.state.read();

        let mut after_release_available_core = after_release_lock
            .available_cores
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        after_release_available_core.sort();
        let after_release_unit_id_mapping = after_release_lock.unit_id_mapping.clone();
        let after_release_type_mapping = after_release_lock.work_type_mapping.clone();
        drop(after_release_lock);

        assert_eq!(after_release_available_core, before_available_core);
        assert_eq!(after_release_unit_id_mapping, before_unit_id_mapping);
        assert_eq!(after_release_type_mapping, before_type_mapping);
    }

    #[test]
    fn test_acquire_error_message() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let init_id_1 =
            <CUID>::from_hex("54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea")
                .unwrap();
        let init_id_2 =
            <CUID>::from_hex("1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0")
                .unwrap();
        let init_id_3 =
            <CUID>::from_hex("271e0e06fdae1f0237055e78f5804416fd9ebb9ca5b52ae360d8124cde220dae")
                .unwrap();
        let persistent_state = PersistentCoreDistributorState {
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
        let (manager, _task) = PersistentCoreDistributor::make_instance_with_task(
            temp_dir.into_path(),
            persistent_state.into(),
            AcquireStrategy::Strict,
        );

        manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: vec![init_id_2],
                worker_type: WorkType::Deal,
            })
            .unwrap();

        let result = manager.acquire_worker_cores(AcquireRequest {
            unit_ids: vec![init_id_3],
            worker_type: WorkType::Deal,
        });

        let expected = "Couldn't assign core: no free cores left. \
        Required: 1, \
        available: 0, \
        acquire_request: { unit_ids: [271e0e06fdae1f0237055e78f5804416fd9ebb9ca5b52ae360d8124cde220dae], worker_type: Deal }, \
        current assignment: [2 -> 1cce3d08f784b11d636f2fb55adf291d43c2e9cbe7ae7eeb2d0301a96be0a3a0, 3 -> 54ae1b506c260367a054f80800a545f23e32c6bc4a8908c9a794cb8dad23e5ea]".to_string();
        assert_eq!(expected, result.unwrap_err().to_string());
    }

    #[test]
    fn test_wrong_range() {
        let cpu_topology = mocked_topology();
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        let range = CoreRange::from_str("0-16384").unwrap();

        let result = PersistentCoreDistributor::from_path(
            temp_dir.path().join("test.toml"),
            2,
            range,
            AcquireStrategy::Strict,
            &cpu_topology,
        );

        assert!(result.is_err());
        assert_eq!(
            result.err().map(|err| err.to_string()),
            Some("The specified CPU range exceeds the available CPU count".to_string())
        );
    }

    #[test]
    fn test_reassignment() {
        let cpu_topology = mocked_topology();

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let system_cpu_count = 1;
        let (manager, _task) = PersistentCoreDistributor::from_path(
            temp_dir.path().join("test.toml"),
            system_cpu_count,
            CoreRange::from_str("0-7").unwrap(),
            AcquireStrategy::Strict,
            &cpu_topology,
        )
        .unwrap();

        let unit_ids_count = 7;
        let unit_ids: Vec<CUID> = (0..unit_ids_count)
            .map(|_| {
                let mut rng = rand::thread_rng();
                let bytes: [u8; 32] = rng.gen();
                CUID::new(bytes)
            })
            .collect();

        let assignment = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::CapacityCommitment,
            })
            .unwrap();
        assert_eq!(assignment.logical_core_ids().len(), unit_ids_count * 2);
        assert_eq!(assignment.cuid_cores.len(), unit_ids_count);

        let assignment = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::Deal,
            })
            .unwrap();
        assert_eq!(assignment.logical_core_ids().len(), unit_ids_count * 2);
        assert_eq!(assignment.cuid_cores.len(), unit_ids_count);
    }

    #[test]
    fn test_workload_assign_error() {
        let cpu_topology = mocked_topology();

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let system_cpu_count = 1;
        let (manager, _task) = PersistentCoreDistributor::from_path(
            temp_dir.path().join("test.toml"),
            system_cpu_count,
            CoreRange::from_str("0-7").unwrap(),
            AcquireStrategy::Strict,
            &cpu_topology,
        )
        .unwrap();

        let unit_ids_count = 7;
        let unit_ids: Vec<CUID> = (0..unit_ids_count)
            .map(|_| {
                let mut rng = rand::thread_rng();
                let bytes: [u8; 32] = rng.gen();
                CUID::new(bytes)
            })
            .collect();

        let assignment = manager
            .acquire_worker_cores(AcquireRequest {
                unit_ids: unit_ids.clone(),
                worker_type: WorkType::CapacityCommitment,
            })
            .unwrap();
        assert_eq!(assignment.logical_core_ids().len(), unit_ids_count * 2);
        assert_eq!(assignment.cuid_cores.len(), unit_ids_count);
        let unit_ids: Vec<CUID> = (0..unit_ids_count)
            .map(|_| {
                let mut rng = rand::thread_rng();
                let bytes: [u8; 32] = rng.gen();
                CUID::new(bytes)
            })
            .collect();

        let result = manager.acquire_worker_cores(AcquireRequest {
            unit_ids: unit_ids.clone(),
            worker_type: WorkType::Deal,
        });

        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                AcquireError::NotFoundAvailableCores {
                    required,
                    available,
                    ..
                } => {
                    assert_eq!(required, unit_ids_count);
                    assert_eq!(available, 0);
                }
            }
        }
    }
}
