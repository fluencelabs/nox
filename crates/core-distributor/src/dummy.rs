use crate::errors::AcquireError;
use crate::types::{AcquireRequest, Assignment, Cores, SystemAssignment};
use crate::{CoreDistributor, Map};
use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use fxhash::FxBuildHasher;

pub struct DummyCoreDistibutor;

impl Default for DummyCoreDistibutor {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyCoreDistibutor {
    pub fn new() -> Self {
        Self {}
    }
}

impl CoreDistributor for DummyCoreDistibutor {
    fn acquire_worker_cores(
        &self,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut data = Map::with_hasher(FxBuildHasher::default());
        for unit_id in acquire_request.unit_ids {
            data.insert(
                unit_id,
                Cores {
                    physical_core_id: PhysicalCoreId::new(0),
                    logical_core_ids: vec![LogicalCoreId::new(0)],
                },
            );
        }

        Ok(Assignment::new(data))
    }

    fn release_worker_cores(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> SystemAssignment {
        SystemAssignment::new(vec![PhysicalCoreId::new(0)], vec![LogicalCoreId::new(0)])
    }
}
