use crate::errors::AcquireError;
use crate::manager::CoreManagerFunctions;
use crate::types::{AcquireRequest, Assignment};
use crate::Map;
use async_trait::async_trait;
use ccp_shared::types::{LogicalCoreId, PhysicalCoreId, CUID};
use fxhash::FxBuildHasher;
use rand::prelude::IteratorRandom;
use std::collections::BTreeSet;

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
