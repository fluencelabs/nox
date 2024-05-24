use std::collections::HashMap;

use ccp_shared::types::{PhysicalCoreId, CUID};
use enum_dispatch::enum_dispatch;
use fxhash::FxBuildHasher;

use crate::distributor::CoreDistributorState;
use crate::errors::{AcquireError, CurrentAssignment};
use crate::types::{AcquireRequest, Assignment, Cores};
use crate::Map;

#[enum_dispatch]
pub(crate) trait AcquireStrategyOperations {
    fn acquire(
        &self,
        state: &mut CoreDistributorState,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError>;
}

pub enum AcquireStrategy {
    Strict,
    RoundRobin,
}

impl From<AcquireStrategy> for AcquireStrategyInner {
    fn from(value: AcquireStrategy) -> Self {
        match value {
            AcquireStrategy::Strict => AcquireStrategyInner::Strict(StrictAcquireStrategy),
            AcquireStrategy::RoundRobin => {
                AcquireStrategyInner::RoundRobin(RoundRobinAcquireStrategy)
            }
        }
    }
}

#[enum_dispatch(AcquireStrategyOperations)]
pub(crate) enum AcquireStrategyInner {
    Strict(StrictAcquireStrategy),
    RoundRobin(RoundRobinAcquireStrategy),
}

pub(crate) struct StrictAcquireStrategy;

impl AcquireStrategyOperations for StrictAcquireStrategy {
    fn acquire(
        &self,
        state: &mut CoreDistributorState,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut cuid_cores: Map<CUID, Cores> = HashMap::with_capacity_and_hasher(
            acquire_request.unit_ids.len(),
            FxBuildHasher::default(),
        );

        let mut result_physical_core_ids = Vec::new();
        let mut result_logical_core_ids = Vec::new();
        let worker_unit_type = acquire_request.worker_type;

        let available = state.available_cores.len();

        let core_usage = acquire_request
            .unit_ids
            .into_iter()
            .map(|unit_id| {
                (
                    unit_id,
                    state.unit_id_mapping.get_by_right(&unit_id).cloned(),
                )
            })
            .collect::<Vec<_>>();

        let required = core_usage.iter().filter(|(_, core)| core.is_none()).count();

        if required > available {
            let current_assignment: Vec<(PhysicalCoreId, CUID)> = state
                .unit_id_mapping
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect();
            return Err(AcquireError::NotFoundAvailableCores {
                required,
                available,
                current_assignment: CurrentAssignment::new(current_assignment),
            });
        }

        for (unit_id, physical_core_id) in core_usage {
            let physical_core_id = match physical_core_id {
                None => {
                    // SAFETY: this should never happen because we already checked the availability of cores
                    let core_id = state
                        .available_cores
                        .pop_back()
                        .expect("Unexpected state. Should not be empty never");
                    state.unit_id_mapping.insert(core_id, unit_id);
                    state
                        .work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    core_id
                }
                Some(core_id) => {
                    state
                        .work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    core_id
                }
            };
            result_physical_core_ids.push(physical_core_id);

            // SAFETY: The physical core always has corresponding logical ids,
            // unit_id_mapping can't have a wrong physical_core_id
            let logical_core_ids = state
                .cores_mapping
                .get_vec(&physical_core_id)
                .cloned()
                .expect("Unexpected state. Should not be empty never");

            for logical_core in logical_core_ids.iter() {
                result_logical_core_ids.push(*logical_core);
            }

            cuid_cores.insert(
                unit_id,
                Cores {
                    physical_core_id,
                    logical_core_ids,
                },
            );
        }

        Ok(Assignment::new(
            result_physical_core_ids,
            result_logical_core_ids,
            cuid_cores,
        ))
    }
}

pub(crate) struct RoundRobinAcquireStrategy;

impl AcquireStrategyOperations for RoundRobinAcquireStrategy {
    fn acquire(
        &self,
        state: &mut CoreDistributorState,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
        let mut result_physical_core_ids = Vec::new();
        let mut result_logical_core_ids = Vec::new();
        let mut cuid_cores: Map<CUID, Cores> = HashMap::with_capacity_and_hasher(
            acquire_request.unit_ids.len(),
            FxBuildHasher::default(),
        );
        let worker_unit_type = acquire_request.worker_type;
        for unit_id in acquire_request.unit_ids {
            let physical_core_id = state.unit_id_mapping.get_by_right(&unit_id).cloned();
            let physical_core_id = match physical_core_id {
                None => {
                    // SAFETY: this should never happen because after the pop operation, we push it back
                    let core_id = state
                        .available_cores
                        .pop_front()
                        .expect("Unexpected state. Should not be empty never");
                    state.unit_id_mapping.insert(core_id, unit_id);
                    state
                        .work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    state.available_cores.push_back(core_id);
                    core_id
                }
                Some(core_id) => {
                    state
                        .work_type_mapping
                        .insert(unit_id, worker_unit_type.clone());
                    core_id
                }
            };
            result_physical_core_ids.push(physical_core_id);

            // SAFETY: The physical core always has corresponding logical ids,
            // unit_id_core_mapping can't have a wrong physical_core_id
            let logical_core_ids = state
                .cores_mapping
                .get_vec(&physical_core_id)
                .cloned()
                .expect("Unexpected state. Should not be empty never");

            for logical_core in logical_core_ids.iter() {
                result_logical_core_ids.push(*logical_core);
            }

            cuid_cores.insert(
                unit_id,
                Cores {
                    physical_core_id,
                    logical_core_ids,
                },
            );
        }
        Ok(Assignment::new(
            result_physical_core_ids,
            result_logical_core_ids,
            cuid_cores,
        ))
    }
}
