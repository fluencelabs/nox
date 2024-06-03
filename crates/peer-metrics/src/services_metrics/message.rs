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
