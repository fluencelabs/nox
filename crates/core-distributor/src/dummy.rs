use crate::errors::AcquireError;
use crate::types::{AcquireRequest, Assignment};
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
    fn acquire_worker_core(
        &self,
        _acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        Ok(Assignment::new(
            vec![PhysicalCoreId::new(0)],
            vec![LogicalCoreId::new(0)],
            Map::with_hasher(FxBuildHasher::default()),
        ))
    }

    fn release(&self, _unit_ids: &[CUID]) {}

    fn get_system_cpu_assignment(&self) -> Assignment {
        Assignment::new(
            vec![PhysicalCoreId::new(0)],
            vec![LogicalCoreId::new(0)],
            Map::with_hasher(FxBuildHasher::default()),
        )
    }
}
