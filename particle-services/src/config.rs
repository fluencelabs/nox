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

use fs_utils::{create_dirs, set_write_only, to_abs_path};

use bytesize::ByteSize;
use cid_utils::Hash;
use fluence_app_service::WasmtimeConfig;
use libp2p_identity::PeerId;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ParticleAppServicesConfig {
    /// Peer id of the current node
    pub local_peer_id: PeerId,
    /// Path of the blueprint directory containing blueprints and wasm modules
    pub blueprint_dir: PathBuf,
    /// Opaque environment variables to be passed on each service creation
    /// TODO: isolate envs of different modules (i.e., module A shouldn't access envs of module B)
    pub envs: HashMap<String, String>,
    /// Persistent working dir for services
    pub persistent_work_dir: PathBuf,
    /// Ephemeral working dir for services
    pub ephemeral_work_dir: PathBuf,
    /// Dir to store .wasm modules and their configs
    pub modules_dir: PathBuf,
    /// Dir to persist info about running services
    pub services_dir: PathBuf,
    /// Dir to store directories shared between services
    /// in the span of a single particle execution  
    pub particles_vault_dir: PathBuf,
    /// key that could manage services
    pub management_peer_id: PeerId,
    /// key to manage builtins services initialization
    pub builtins_management_peer_id: PeerId,
    /// Default heap size in bytes available for the module unless otherwise specified.
    pub default_service_memory_limit: Option<ByteSize>,
    /// List of allowed effector modules by CID
    pub allowed_effectors: HashMap<Hash, HashMap<String, PathBuf>>,
    /// Mapping of binary names to their paths for mounted binaries used in developer mode
    pub mounted_binaries_mapping: HashMap<String, PathBuf>,
    /// Is in the developer mode
    pub is_dev_mode: bool,
    /// config for the wasmtime backend
    pub wasm_backend_config: WasmBackendConfig,
}

impl ParticleAppServicesConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        local_peer_id: PeerId,
        persistent_dir: PathBuf,
        ephemeral_dir: PathBuf,
        particles_vault_dir: PathBuf,
        envs: HashMap<String, String>,
        management_peer_id: PeerId,
        builtins_management_peer_id: PeerId,
        default_service_memory_limit: Option<ByteSize>,
        allowed_effectors: HashMap<Hash, HashMap<String, String>>,
        mounted_binaries_mapping: HashMap<String, String>,
        is_dev_mode: bool,
        wasm_backend_config: WasmBackendConfig,
    ) -> Result<Self, std::io::Error> {
        let persistent_dir = to_abs_path(persistent_dir);
        let ephemeral_dir = to_abs_path(ephemeral_dir);

        let allowed_effectors = allowed_effectors
            .into_iter()
            .map(|(cid, effector)| {
                let effector = effector
                    .into_iter()
                    .map(|(name, path_str)| {
                        let path = Path::new(&path_str);
                        match path.try_exists() {
                            Err(err) => tracing::warn!(
                                "cannot check binary `{path_str}` for effector `{cid}`: {err}"
                            ),
                            Ok(false) => tracing::warn!(
                                "binary `{path_str}` for effector `{cid}` does not exist"
                            ),
                            _ => {}
                        };
                        (name, path.to_path_buf())
                    })
                    .collect::<_>();
                (cid, effector)
            })
            .collect::<_>();

        let mounted_binaries_mapping = if !is_dev_mode {
            HashMap::new()
        } else {
            mounted_binaries_mapping
                .into_iter()
                .map(|(name, path_str)| {
                    let path = Path::new(&path_str);
                    match path.try_exists() {
                        Err(err) => tracing::warn!("cannot check binary `{path_str}`: {err}"),
                        Ok(false) => tracing::warn!("binary `{path_str}` does not exist"),
                        _ => {}
                    };
                    (name, path.to_path_buf())
                })
                .collect::<_>()
        };

        let this = Self {
            local_peer_id,
            blueprint_dir: config_utils::blueprint_dir(&persistent_dir),
            persistent_work_dir: config_utils::workdir(&persistent_dir),
            ephemeral_work_dir: config_utils::workdir(&ephemeral_dir),
            modules_dir: config_utils::modules_dir(&persistent_dir),
            services_dir: config_utils::services_dir(&persistent_dir),
            particles_vault_dir,
            envs,
            management_peer_id,
            builtins_management_peer_id,
            default_service_memory_limit,
            allowed_effectors,
            mounted_binaries_mapping,
            is_dev_mode,
            wasm_backend_config,
        };

        create_dirs(&[
            &this.blueprint_dir,
            &this.persistent_work_dir,
            &this.ephemeral_work_dir,
            &this.modules_dir,
            &this.services_dir,
            &this.particles_vault_dir,
        ])?;

        set_write_only(&this.particles_vault_dir)?;

        Ok(this)
    }
}

#[derive(Debug, Clone)]
pub struct WasmBackendConfig {
    /// Configures whether DWARF debug information will be emitted during compilation.
    pub debug_info: bool,
    /// Configures whether the errors from the VM should collect the wasm backtrace and parse debug info.
    pub wasm_backtrace: bool,
    /// Configures the size of the stacks used for asynchronous execution.
    pub async_wasm_stack: usize,
    /// Configures the maximum amount of stack space available for executing WebAssembly code.
    pub max_wasm_stack: usize,
    /// Enables the epoch interruption mechanism.
    pub epoch_interruption_duration: Option<Duration>,
}

impl From<WasmBackendConfig> for WasmtimeConfig {
    fn from(value: WasmBackendConfig) -> Self {
        let mut config = WasmtimeConfig::default();
        config
            .debug_info(value.debug_info)
            .wasm_backtrace(value.wasm_backtrace)
            .epoch_interruption(true)
            .async_wasm_stack(value.async_wasm_stack)
            .max_wasm_stack(value.max_wasm_stack);
        config
    }
}

impl Default for WasmBackendConfig {
    fn default() -> Self {
        Self {
            debug_info: true,
            wasm_backtrace: true,
            async_wasm_stack: 4 * 1024 * 1024,
            max_wasm_stack: 2 * 1024 * 1024,
            epoch_interruption_duration: Some(Duration::from_secs(1)),
        }
    }
}
