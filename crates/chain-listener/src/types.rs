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
use ccp_shared::types::{PhysicalCoreId, CUID};

#[derive(Debug)]
pub struct CUGroups {
    /// Already started units involved in CC and not having less than MIN_PROOFS_PER_EPOCH proofs in the current epoch
    pub priority_units: Vec<CUID>,
    /// Already started units involved in CC and found at least MIN_PROOFS_PER_EPOCH proofs,
    /// but less that MAX_PROOFS_PER_EPOCH proofs in the current epoch
    pub non_priority_units: Vec<CUID>,
    /// Units in CC that is not active yet and can't produce proofs in the current epoch
    pub pending_units: Vec<CUID>,
    /// Already started units involved in CC and having more than MAX_PROOFS_PER_EPOCH proofs in the current epoch
    pub finished_units: Vec<CUID>,
}

impl CUGroups {
    pub fn all_min_proofs_found(&self) -> bool {
        self.priority_units.is_empty()
    }

    pub fn all_max_proofs_found(&self) -> bool {
        self.non_priority_units.is_empty()
    }
}

#[derive(Debug)]
pub struct PhysicalCoreGroups {
    pub priority_cores: Vec<PhysicalCoreId>,
    pub non_priority_cores: Vec<PhysicalCoreId>,
    pub pending_cores: Vec<PhysicalCoreId>,
    pub finished_cores: Vec<PhysicalCoreId>,
}
