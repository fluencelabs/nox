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

#![feature(try_blocks)]
#![feature(result_option_inspect)]

mod deployer;
mod distro;

use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

pub use deployer::Deployer;
pub use distro::SystemServiceDistros;
pub use distro::Versions;

use fluence_app_service::{TomlMarineConfig, TomlValue};
use fluence_spell_dtos::trigger_config::TriggerConfig;
use serde_json::Value;

type ServiceName = String;
type FunctionName = String;

/// Status of package deployment for each services and spells of the package
#[derive(Clone, Debug)]
pub struct Deployment {
    /// Statuses of spells deployment
    pub spells: HashMap<String, ServiceStatus>,
    /// Statuses of services deployment
    pub services: HashMap<String, ServiceStatus>,
}

/// Call service functions. Accepts
/// - service name
/// - function name
/// - function arguments
///
/// Restriction:
/// The functions called via this callback must return a result with execution status in field `status: bool`
/// and error message in the field `error: string`.
/// Otherwise, the output will be consider invalid.
#[async_trait]
pub trait CallService: Send + Sync {
    async fn call(
        &self,
        service_name: ServiceName,
        function_name: FunctionName,
        args: Vec<Value>,
    ) -> eyre::Result<()>;
}

/// Initialization function to initialize services
/// - accepts `DeploymentStatus` of services and spells to be able to update or initialize the services
/// - accepts `CallService` to be able to call installed services for initialization.
#[async_trait]
pub trait InitService: Send + Sync {
    async fn init(
        &self,
        call_service: &dyn CallService,
        deployment: Deployment,
    ) -> eyre::Result<()>;
}

/// Package distribution description
/// Contains enough information about all services and spells used by the package for installation
/// It's not the same thing as a worker since this kind of package doesn't require to be installed
/// in a separate scope
#[derive(Clone)]
pub struct PackageDistro {
    /// High-level name of the package.
    /// For the system services the field is supposed to be a string of `ServiceKey` enum
    pub name: String,
    /// Version of the package, the field is used to display versions of used system services
    pub version: &'static str,
    /// List services needed by the package
    pub services: Vec<ServiceDistro>,
    /// List of spells needed by the package
    pub spells: Vec<SpellDistro>,
    /// Optionally, initialization function for the services.
    pub init: Option<Arc<Box<dyn InitService>>>,
}

impl fmt::Debug for PackageDistro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackageDistro")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("services", &self.services)
            .field("spells", &self.spells)
            .field("init", &"{...}".to_string())
            .finish()
    }
}

/// Service distribution description that provides enough information for the service installation.
#[derive(Clone)]
pub struct ServiceDistro {
    /// WASM modules of the service by their names
    pub modules: HashMap<&'static str, &'static [u8]>,
    /// Marine config of the service
    pub config: TomlMarineConfig,
    /// High-level names of the service used as an alias
    pub name: String,
}

impl fmt::Debug for ServiceDistro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceDistro")
            .field("modules", &self.modules.keys())
            .field("config", &self.config)
            .field("name", &self.name)
            .finish()
    }
}

/// Spell distribution description that provides enough information for the spell installation.
#[derive(Clone)]
pub struct SpellDistro {
    /// The name of the spell which is also used as an alias for the spell
    pub name: String,
    /// The AIR script of the spell
    pub air: &'static str,
    /// Initial values for the KV storage of the spell
    /// Note that these values are saved as JSON strings
    pub kv: HashMap<&'static str, Value>,
    /// The trigger config for the spell
    pub trigger_config: TriggerConfig,
}

impl fmt::Debug for SpellDistro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpellDistro")
            .field("name", &self.name)
            .field(
                "air",
                &format!("{}...", self.air.chars().take(20).collect::<String>()),
            )
            .field("kv", &self.kv.keys())
            .field("trigger_config", &self.trigger_config)
            .finish()
    }
}

/// A status of a service/spell after deployment
#[derive(Clone, Debug)]
pub enum ServiceStatus {
    /// Id of a newly created service
    Created(String),
    /// Id of a already existing service
    Existing(String),
}

/// Override a binary path to a binary for a module in the service configuration
fn apply_binary_path_override(
    config: &mut TomlMarineConfig,
    // Name of the module for which we override the path
    module_name: &str,
    // The name of the binary to override
    binary_name: &str,
    // Path to the binary to use instead
    binary_path: PathBuf,
) {
    if let Some(module_config) = config.module.iter_mut().find(|p| p.name == module_name) {
        if let Some(mounted_binaries) = &mut module_config.config.mounted_binaries {
            if let Some(path) = mounted_binaries.get_mut(binary_name) {
                *path = TomlValue::String(binary_path.display().to_string());
            }
        }
    }
}
