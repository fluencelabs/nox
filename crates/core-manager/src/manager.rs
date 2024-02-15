use enum_dispatch::enum_dispatch;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash};

use fxhash::{FxBuildHasher, FxHasher};
use hwloc2::{ObjectType, Topology, TypeDepthError};
use parking_lot::RwLock;
use range_set_blaze::RangeSetBlaze;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::core_range::CoreRange;

extern crate hwloc2;

type Map<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher>>;
type MultiMap<K, V> = multimap::MultiMap<K, V, BuildHasherDefault<FxHasher>>;
type BiMap<K, V> =
    bimap::BiHashMap<K, V, BuildHasherDefault<FxHasher>, BuildHasherDefault<FxHasher>>;

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
pub struct PhysicalCoreId(usize);

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
pub struct LogicalCoreId(pub usize);

#[enum_dispatch]
pub trait CoreManagerFunctions {
    fn assign_worker_core(&self, assign_request: AssignRequest) -> Result<Assignment, AssignError>;
    fn system_cpu_assignment(&self) -> Assignment;
    fn free(&mut self, unit_ids: Vec<UnitId>);
}

#[enum_dispatch(CoreManagerFunctions)]
pub enum CoreManager {
    Default(DefaultCoreManager),
    Dummy(DummyCoreManager),
}

pub struct DefaultCoreManager {
    inner: RwLock<InnerState>,
}

struct InnerState {
    // mapping between physical and logical cores
    cores_mapping: MultiMap<PhysicalCoreId, LogicalCoreId>,
    //
    system_cores: Vec<PhysicalCoreId>,

    // free physical cores
    available_cores: Vec<PhysicalCoreId>,
    // mapping between physical core id and unit id
    unit_id_mapping: BiMap<PhysicalCoreId, UnitId>,

    type_mapping: Map<UnitId, WorkerType>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct UnitId(String);

impl From<String> for UnitId {
    fn from(value: String) -> Self {
        UnitId(value)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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
pub enum AssignError {
    #[error("Not found free cores")]
    NotFoundAvailableCores,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Assignment {
    pub physical_core_ids: Vec<PhysicalCoreId>,
    pub logical_core_ids: Vec<LogicalCoreId>,
}

impl DefaultCoreManager {
    pub fn new(system_cpu_count: usize, cpus_range: Option<CoreRange>) -> Result<Self, Error> {
        let core_range = cpus_range
            .map(|range| range.0)
            .unwrap_or(RangeSetBlaze::from_iter(0..num_cpus::get_physical()));

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

        let system_cores: Vec<PhysicalCoreId> = available_cores.drain(..system_cpu_count).collect();

        let unit_id_mapping = BiMap::with_capacity_and_hashers(
            available_core_count,
            FxBuildHasher::default(),
            FxBuildHasher::default(),
        );

        let type_mapping =
            Map::with_capacity_and_hasher(available_core_count, FxBuildHasher::default());

        let inner = RwLock::new(InnerState {
            cores_mapping,
            system_cores,
            available_cores,
            unit_id_mapping,
            type_mapping,
        });
        Ok(Self { inner })
    }
}

impl CoreManagerFunctions for DefaultCoreManager {
    fn assign_worker_core(&self, assign_request: AssignRequest) -> Result<Assignment, AssignError> {
        let mut lock = self.inner.write();
        let mut physical_core_ids = Vec::with_capacity(assign_request.unit_ids.len());
        let mut logical_core_ids = vec![];
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
            physical_core_ids.push(core_id.clone());
            let local_physical_core_ids = lock
                .cores_mapping
                .get_vec(&core_id)
                .expect("Can't be empty");
            logical_core_ids.extend_from_slice(local_physical_core_ids);
        }
        Ok(Assignment {
            physical_core_ids,
            logical_core_ids,
        })
    }

    fn system_cpu_assignment(&self) -> Assignment {
        let lock = self.inner.read();
        let mut logical_core_ids = vec![];
        for core in &lock.system_cores {
            let core_ids = lock.cores_mapping.get_vec(core).cloned().unwrap();
            logical_core_ids.extend_from_slice(core_ids.as_slice());
        }
        Assignment {
            physical_core_ids: lock.system_cores.clone(),
            logical_core_ids,
        }
    }

    fn free(&mut self, unit_ids: Vec<UnitId>) {
        let mut lock = self.inner.write();
        for unit_id in unit_ids {
            if let Some(core_id) = lock.unit_id_mapping.get_by_right(&unit_id).cloned() {
                lock.available_cores.push(core_id.clone());
                lock.unit_id_mapping.remove_by_left(&core_id);
                lock.type_mapping.remove(&unit_id);
            }
        }
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

impl CoreManagerFunctions for DummyCoreManager {
    fn assign_worker_core(
        &self,
        _assign_request: AssignRequest,
    ) -> Result<Assignment, AssignError> {
        Ok(self.all_cores())
    }

    fn system_cpu_assignment(&self) -> Assignment {
        self.all_cores()
    }

    fn free(&mut self, _unit_ids: Vec<UnitId>) {}
}

#[cfg(test)]
mod tests {
    use crate::manager::{
        AssignRequest, CoreManagerFunctions, DefaultCoreManager, UnitId, WorkerType,
    };

    #[test]
    fn test_assignment_and_switching() {
        let manager = DefaultCoreManager::new(2, None).unwrap();
        let unit_ids = vec![UnitId("1".to_string()), UnitId("2".to_string())];
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
