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

use crate::ServiceType;
use fluence_app_service::MemoryStats;
use std::collections::HashMap;

pub type ModuleName = String;
pub type MemorySize = u64;

/// Service function call stats to store in memory
#[derive(Debug)]
pub enum ServiceCallStats {
    Success {
        memory_delta_bytes: f64,
        call_time_sec: f64,
        lock_wait_time_sec: f64,
        timestamp: u64,
    },
    Fail {
        timestamp: u64,
    },
}

/// Messages to the metrics backend
#[derive(Debug)]
pub enum ServiceMetricsMsg {
    Memory {
        service_id: String,
        service_type: ServiceType,
        memory_stat: ServiceMemoryStat,
    },
    CallStats {
        service_id: String,
        function_name: String,
        stats: ServiceCallStats,
    },
}

#[derive(Default, Debug)]
pub struct ServiceMemoryStat {
    /// Memory used by the service
    pub used_mem: MemorySize,
    /// Memory used by the modules that belongs to the service
    pub modules_stats: HashMap<ModuleName, MemorySize>,
}

impl ServiceMemoryStat {
    pub fn new(stats: &MemoryStats) -> ServiceMemoryStat {
        let mut modules_stats = HashMap::new();
        let mut used_mem: MemorySize = 0;
        for stat in &stats.modules {
            modules_stats.insert(stat.name.to_string(), stat.memory_size as MemorySize);
            used_mem += stat.memory_size as MemorySize;
        }
        ServiceMemoryStat {
            used_mem,
            modules_stats,
        }
    }
}
