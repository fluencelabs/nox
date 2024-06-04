/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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

    fn release(&self, state: &mut CoreDistributorState, unit_ids: &[CUID]);
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

        let worker_unit_type = &acquire_request.worker_type;

        let available = state.available_cores.len();

        let core_allocation = acquire_request
            .unit_ids
            .iter()
            .map(|&unit_id| {
                (
                    unit_id,
                    //TODO: introduce a new enum to make code self-documented.
                    state.unit_id_mapping.get_by_right(&unit_id).cloned(),
                )
            })
            .collect::<Vec<_>>();

        let required = core_allocation
            .iter()
            .filter(|(_, core)| core.is_none())
            .count();

        if required > available {
            let current_assignment: Vec<(PhysicalCoreId, CUID)> = state
                .unit_id_mapping
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect();
            return Err(AcquireError::NotFoundAvailableCores {
                required,
                available,
                acquire_request: acquire_request.clone(),
                current_assignment: CurrentAssignment::new(current_assignment),
            });
        }

        for (unit_id, physical_core_id) in core_allocation {
            let physical_core_id = match physical_core_id {
                None => {
                    // SAFETY: this should never happen because we already checked the availability of cores
                    let core_id = state
                        .available_cores
                        .pop_back()
                        .expect("Unexpected state. Should not be empty never");
                    state.unit_id_mapping.insert(core_id, unit_id);
                    state.work_type_mapping.insert(unit_id, *worker_unit_type);
                    core_id
                }
                Some(core_id) => {
                    state.work_type_mapping.insert(unit_id, *worker_unit_type);
                    core_id
                }
            };

            // SAFETY: The physical core always has corresponding logical ids,
            // unit_id_mapping can't have a wrong physical_core_id
            let logical_core_ids = state
                .cores_mapping
                .get_vec(&physical_core_id)
                .cloned()
                .expect("Unexpected state. Should not be empty never");

            cuid_cores.insert(
                unit_id,
                Cores {
                    physical_core_id,
                    logical_core_ids,
                },
            );
        }

        Ok(Assignment::new(cuid_cores))
    }

    fn release(&self, state: &mut CoreDistributorState, unit_ids: &[CUID]) {
        for unit_id in unit_ids {
            if let Some((physical_core_id, _)) = state.unit_id_mapping.remove_by_right(unit_id) {
                state.available_cores.push_back(physical_core_id);
                state.work_type_mapping.remove(unit_id);
            }
        }
    }
}

pub(crate) struct RoundRobinAcquireStrategy;

impl AcquireStrategyOperations for RoundRobinAcquireStrategy {
    fn acquire(
        &self,
        state: &mut CoreDistributorState,
        acquire_request: AcquireRequest,
    ) -> Result<Assignment, AcquireError> {
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
                    state.work_type_mapping.insert(unit_id, worker_unit_type);
                    state.available_cores.push_back(core_id);
                    core_id
                }
                Some(core_id) => {
                    state.work_type_mapping.insert(unit_id, worker_unit_type);
                    core_id
                }
            };

            // SAFETY: The physical core always has corresponding logical ids,
            // unit_id_core_mapping can't have a wrong physical_core_id
            let logical_core_ids = state
                .cores_mapping
                .get_vec(&physical_core_id)
                .cloned()
                .expect("Unexpected state. Should not be empty never");

            cuid_cores.insert(
                unit_id,
                Cores {
                    physical_core_id,
                    logical_core_ids,
                },
            );
        }
        Ok(Assignment::new(cuid_cores))
    }

    fn release(&self, state: &mut CoreDistributorState, unit_ids: &[CUID]) {
        for unit_id in unit_ids {
            state.unit_id_mapping.remove_by_right(unit_id);
            state.work_type_mapping.remove(unit_id);
        }
    }
}
