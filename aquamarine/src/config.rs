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

use fs_utils::to_abs_path;
use libp2p::PeerId;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub current_peer_id: PeerId,
    /// Path to AIR interpreter .wasm file (aquamarine.wasm)
    pub air_interpreter: PathBuf,
    /// Maximum heap size in bytes available for the interpreter.
    pub max_heap_size: Option<u64>,
    /// Maximum AIR script size in bytes.
    pub air_size_limit: Option<u64>,
    /// Maximum particle size in bytes.
    pub particle_size_limit: Option<u64>,
    /// Maximum call result size in bytes.
    pub call_result_size_limit: Option<u64>,
    /// A knob to enable/disable hard limits behavior in AquaVM.
    pub hard_limit_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct VmPoolConfig {
    /// Number of VMs to create
    pub pool_size: usize,
    /// Timeout of a particle execution
    pub execution_timeout: Duration,
}

impl VmConfig {
    pub fn new(
        current_peer_id: PeerId,
        air_interpreter: PathBuf,
        max_heap_size: Option<u64>,
        air_size_limit: Option<u64>,
        particle_size_limit: Option<u64>,
        call_result_size_limit: Option<u64>,
        hard_limit_enabled: bool,
    ) -> Self {
        Self {
            current_peer_id,
            air_interpreter,
            max_heap_size,
            air_size_limit,
            particle_size_limit,
            call_result_size_limit,
            hard_limit_enabled,
        }
    }
}

impl VmPoolConfig {
    pub fn new(pool_size: usize, execution_timeout: Duration) -> Self {
        Self {
            pool_size,
            execution_timeout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataStoreConfig {
    /// Dir for the interpreter to persist particle data
    /// to merge it between particles of the same particle_id
    pub particles_dir: PathBuf,
    /// Dir to store directories shared between services
    /// in the span of a single particle execution
    pub particles_vault_dir: PathBuf,
    /// Dir to store particles data of AquaVM performance anomalies
    pub particles_anomaly_dir: PathBuf,
}

impl DataStoreConfig {
    pub fn new(base_dir: PathBuf) -> Self {
        let base_dir = to_abs_path(base_dir);
        Self {
            particles_dir: config_utils::particles_dir(&base_dir),
            particles_vault_dir: config_utils::particles_vault_dir(&base_dir),
            particles_anomaly_dir: config_utils::particles_anomaly_dir(&base_dir),
        }
    }
}
