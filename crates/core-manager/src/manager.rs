use std::hash::{BuildHasherDefault, Hash};

use fxhash::{FxBuildHasher, FxHasher};
use hwloc2::{ObjectType, Topology, TypeDepthError};
use parking_lot::Mutex;
use range_set_blaze::RangeSetBlaze;
use thiserror::Error;

use crate::core_range::CoreRange;

extern crate hwloc2;

type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
pub struct PhysicalCoreId(usize);

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
pub struct LogicalCoreId(usize);

pub struct CoreManager {
    available_cores: Vec<PhysicalCoreId>,
    cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId>,
    cores_state: BiMap<PhysicalCoreId, WorkerUnitType>,
    mutex: Mutex<()>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct UnitId(String);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum WorkerUnitType {
    CC(UnitId),
    Worker(UnitId),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create topology")]
    CreateTopology,
    #[error("Failed to collect cores data {err:?}")]
    CollectCoresData { err: TypeDepthError },
}

#[derive(Debug, Error)]
pub enum AssignError {
    #[error("Not found free cores")]
    NotFoundAvailableCores,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Assignment {
    physical_core_id: PhysicalCoreId,
    logical_core_ids: Vec<LogicalCoreId>,
}

impl CoreManager {
    pub fn new(cpus_range: Option<CoreRange>) -> Result<Self, Error> {
        let core_range = cpus_range
            .map(|range| range.0)
            .unwrap_or(RangeSetBlaze::from_iter(0..num_cpus::get()));

        let topology = Topology::new().ok_or(Error::CreateTopology)?;

        let cores = topology
            .objects_with_type(&ObjectType::Core)
            .map_err(|err| Error::CollectCoresData { err })?;

        let mut cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId> =
            MultiMap::with_capacity_and_hasher(core_range.len() as usize, FxBuildHasher::default());

        for physical_core in cores {
            let logical_cores = physical_core.children();
            for logical_core in logical_cores {
                let logical_core_id = logical_core.os_index() as usize;
                if core_range.contains(logical_core_id) {
                    cores_mapping.insert(
                        PhysicalCoreId(physical_core.os_index() as usize),
                        LogicalCoreId(logical_core_id),
                    )
                }
            }
        }
        let mut available_cores: Vec<PhysicalCoreId> = cores_mapping.keys().cloned().collect();
        available_cores.sort();

        let cores_state = BiMap::with_capacity_and_hashers(
            core_range.len() as usize,
            FxBuildHasher::default(),
            FxBuildHasher::default(),
        );

        Ok(Self {
            cores_mapping,
            available_cores,
            cores_state,
            mutex: Mutex::new(()),
        })
    }

    pub fn assign_worker_core(
        &mut self,
        worker_type: WorkerUnitType,
    ) -> Result<Assignment, AssignError> {
        match worker_type {
            WorkerUnitType::CC(unit_id) => self.switch_or_assign(
                WorkerUnitType::Worker(unit_id.clone()),
                WorkerUnitType::CC(unit_id),
            ),
            WorkerUnitType::Worker(unit_id) => self.switch_or_assign(
                WorkerUnitType::CC(unit_id.clone()),
                WorkerUnitType::Worker(unit_id),
            ),
        }
    }

    fn switch_or_assign(
        &mut self,
        src: WorkerUnitType,
        dsc: WorkerUnitType,
    ) -> Result<Assignment, AssignError> {
        let _guard = self.mutex.lock();
        let state = self.cores_state.get_by_right(&src).cloned();
        let physical_core_id = match state {
            None => {
                let core_id = self.available_cores.pop();
                match core_id {
                    None => return Err(AssignError::NotFoundAvailableCores),
                    Some(core_id) => {
                        self.cores_state.insert(core_id.clone(), dsc);
                        core_id
                    }
                }
            }
            Some(core_id) => {
                self.cores_state.insert(core_id.clone(), dsc);
                core_id
            }
        };
        let logical_core_ids = self
            .cores_mapping
            .get_vec(&physical_core_id)
            .cloned()
            .expect("Can't be empty");

        Ok(Assignment {
            physical_core_id,
            logical_core_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::manager::{UnitId, WorkerUnitType};
    use crate::CoreManager;

    #[test]
    fn test_assignment_and_switching() {
        let mut manager = CoreManager::new(None).unwrap();
        let unit_id = UnitId("1".to_string());
        let assignment_1 = manager
            .assign_worker_core(WorkerUnitType::CC(unit_id.clone()))
            .unwrap();
        let assignment_2 = manager
            .assign_worker_core(WorkerUnitType::Worker(unit_id.clone()))
            .unwrap();
        let assignment_3 = manager
            .assign_worker_core(WorkerUnitType::CC(unit_id))
            .unwrap();
        assert_eq!(assignment_1, assignment_2);
        assert_eq!(assignment_1, assignment_3);
    }
}
