use std::collections::{BTreeSet, VecDeque};

use async_trait::async_trait;
use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use fxhash::FxBuildHasher;
use parking_lot::RwLock;

use crate::errors::{AcquireError, CurrentAssignment};
use crate::manager::CoreManagerFunctions;
use crate::types::{AcquireRequest, Assignment, Cores};
use crate::Map;

#[derive(Default)]
pub struct DummyCoreManager {
    state: RwLock<State>,
}

struct State {
    available: VecDeque<Cores>,
    used: Map<CUID, Cores>,
}

impl Default for State {
    fn default() -> Self {
        let topology: Vec<Cores> = (0..16)
            .map(|index| {
                let physical_core_id = PhysicalCoreId::from(index);
                let logical_core_ids = LogicalCoreId::from(index);
                Cores {
                    physical_core_id,
                    logical_core_ids: vec![logical_core_ids],
                }
            })
            .collect();

        Self {
            available: topology.into_iter().collect(),
            used: Map::with_hasher(FxBuildHasher::default()),
        }
    }
}

impl DummyCoreManager {
    pub fn new(topology: Vec<Cores>) -> Self {
        let state = State {
            available: topology.into_iter().collect(),
            used: Map::with_hasher(FxBuildHasher::default()),
        };
        Self {
            state: RwLock::new(state),
        }
    }
}

#[async_trait]
impl CoreManagerFunctions for DummyCoreManager {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut state = self.state.write();
        let available = state.available.len();
        let required = assign_request.unit_ids.len();
        if required > available {
            return Err(AcquireError::NotFoundAvailableCores {
                required,
                available,
                current_assignment: CurrentAssignment::new(vec![]),
            });
        }

        let mut physical_core_ids = BTreeSet::new();
        let mut logical_core_ids = BTreeSet::new();
        let mut cuid_cores = Map::with_hasher(FxBuildHasher::default());

        for cuid in assign_request.unit_ids {
            let core = state.available.pop_front().unwrap();
            physical_core_ids.insert(core.physical_core_id);
            for logical_core_id in core.logical_core_ids.clone() {
                logical_core_ids.insert(logical_core_id);
            }
            state.used.insert(cuid, core.clone());
            cuid_cores.insert(cuid, core);
        }

        Ok(Assignment {
            physical_core_ids,
            logical_core_ids,
            cuid_cores,
        })
    }

    fn release(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> Assignment {
        let physical_core_ids = (0..num_cpus::get_physical())
            .map(|v| PhysicalCoreId::from(v as u32))
            .collect();
        let logical_core_ids = (0..num_cpus::get())
            .map(|v| LogicalCoreId::from(v as u32))
            .collect();
        Assignment {
            physical_core_ids,
            logical_core_ids,
            cuid_cores: Map::with_hasher(FxBuildHasher::default()),
        }
    }
}

#[async_trait]
impl CoreManagerFunctions for DummyCoreManager {
    fn acquire_worker_core(
        &self,
        assign_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let all_cores = self.all_cores();

        let logical_core_ids: BTreeSet<LogicalCoreId> = BTreeSet::from_iter(
            all_cores
                .logical_core_ids
                .into_iter()
                .choose_multiple(&mut rand::thread_rng(), assign_request.unit_ids.len()),
        );

        Ok(Assignment {
            physical_core_ids: BTreeSet::new(),
            logical_core_ids,
            cuid_cores: Map::with_hasher(FxBuildHasher::default()),
        })
    }

    fn release(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> Assignment {
        self.all_cores()
    }
}
