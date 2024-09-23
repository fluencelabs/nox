/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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

        let worker_unit_type = &acquire_request.work_type;

        let available = state.available_cores.len();

        let core_allocation = acquire_request
            .unit_ids
            .iter()
            .map(|&unit_id| {
                (
                    unit_id,
                    // TODO: introduce a new enum to make code self-documented.
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

        let mut new_core_allocation = Vec::with_capacity(core_allocation.len());
        for (unit_id, physical_core_id) in core_allocation {
            match state.cuid_cache.get(&unit_id) {
                None => new_core_allocation.push((unit_id, physical_core_id)),
                Some(cached_physical_core_id) => {
                    let position = state
                        .available_cores
                        .iter()
                        .position(|core_id| core_id == cached_physical_core_id);
                    match position {
                        None => new_core_allocation.push((unit_id, physical_core_id)),
                        Some(index) => {
                            // SAFETY: this should never happen because we already found position in the previous step
                            let physical_core_id = state
                                .available_cores
                                .remove(index)
                                .expect("Unexpected state. Should not be empty never");

                            state.unit_id_mapping.insert(physical_core_id, unit_id);
                            state.work_type_mapping.insert(unit_id, *worker_unit_type);
                            Self::add_cuid_cores(state, unit_id, physical_core_id, &mut cuid_cores);
                        }
                    }
                }
            }
        }

        for (unit_id, physical_core_id) in new_core_allocation {
            let physical_core_id = match physical_core_id {
                None => {
                    // SAFETY: this should never happen because we already checked the availability of cores
                    let physical_core_id = state
                        .available_cores
                        .pop_back()
                        .expect("Unexpected state. Should not be empty never");
                    state.unit_id_mapping.insert(physical_core_id, unit_id);
                    state.work_type_mapping.insert(unit_id, *worker_unit_type);
                    state.cuid_cache.insert(unit_id, physical_core_id);
                    physical_core_id
                }
                Some(core_id) => {
                    state.work_type_mapping.insert(unit_id, *worker_unit_type);
                    core_id
                }
            };

            Self::add_cuid_cores(state, unit_id, physical_core_id, &mut cuid_cores);
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

impl StrictAcquireStrategy {
    fn add_cuid_cores(
        state: &CoreDistributorState,
        unit_id: CUID,
        physical_core_id: PhysicalCoreId,
        cuid_cores: &mut Map<CUID, Cores>,
    ) {
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
        let worker_unit_type = acquire_request.work_type;
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
