/*
 * Copyright 2020 Fluence Labs Limited
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

use crate::system_services_config::ServiceKey;
use crate::LogFormat;
use clap::error::ErrorKind;
use clap::{Args, Parser};
use config::{ConfigError, Map, Source, Value};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct RootKeyPairArgs {
    #[arg(
        short('k'),
        long("keypair-value"),
        id = "ROOT_KEY_PAIR_VALUE",
        help = "keypair in base64 (conflicts with --keypair-path)",
        value_name = "BYTES",
        help_heading = "Node keypair",
        display_order = 10,
        conflicts_with = "ROOT_KEY_PAIR_PATH",
        conflicts_with = "SECRET_KEY"
    )]
    value: Option<String>,
    #[arg(
        short('p'),
        long("keypair-path"),
        id = "ROOT_KEY_PAIR_PATH",
        help = "keypair path (conflicts with --keypair-value)",
        help_heading = "Node keypair",
        display_order = 11,
        conflicts_with = "ROOT_KEY_PAIR_VALUE",
        conflicts_with = "SECRET_KEY"
    )]
    path: Option<PathBuf>,
    #[arg(
        short('f'),
        long("keypair-format"),
        value_parser(["ed25519", "secp256k1", "rsa"]),
        id = "ROOT_KEY_FORMAT",
        help_heading = "Node keypair",
        display_order = 12,
    )]
    format: Option<String>,

    #[arg(
        short('g'),
        long("gen-keypair"),
        value_parser = clap::value_parser!(bool),
        id = "ROOT_KEY_PAIR_GENERATE",
        help_heading = "Node keypair",
        display_order = 13,
        action =  clap::ArgAction::SetTrue
    )]
    generate_on_absence: Option<bool>,
    #[arg(
        short('y'),
        long,
        id = "SECRET_KEY",
        help_heading = "Node keypair",
        help = "Node secret key in base64 (usually 32 bytes)",
        display_order = 14,
        conflicts_with = "ROOT_KEY_PAIR_PATH",
        conflicts_with = "ROOT_KEY_PAIR_VALUE"
    )]
    secret_key: Option<String>,
}

impl Serialize for RootKeyPairArgs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut struct_serializer = serializer.serialize_struct("KeypairConfig", 4)?;

        if let Some(generate_on_absence) = &self.generate_on_absence {
            struct_serializer.serialize_field("generate_on_absence", generate_on_absence)?;
        }
        if let Some(format) = &self.format {
            struct_serializer.serialize_field("format", format)?;
        }
        if let Some(value) = &self.value {
            struct_serializer.serialize_field("value", &value)?;
        }
        if let Some(value) = &self.path {
            struct_serializer.serialize_field("path", &value)?;
        }
        if let Some(value) = &self.secret_key {
            struct_serializer.serialize_field("secret_key", &value)?;
        }
        struct_serializer.end()
    }
}

#[derive(Args, Debug, Clone, Serialize)]
pub struct LogArgs {
    #[arg(
        long("log-format"),
        id = "LOG_FORMAT",
        help = "logging format",
        help_heading = "Node configuration",
        display_order = 24
    )]
    pub format: Option<LogFormat>,
}

#[derive(Args, Debug, Clone, Serialize)]
pub struct TracingArgs {
    #[arg(
        long("tracing-type"),
        id = "TRACING_TYPE",
        help = "Tracing type",
        help_heading = "Node configuration",
        display_order = 25,
        value_enum
    )]
    #[serde(rename = "type")]
    tpe: Option<TracingType>,

    #[arg(
        long("tracing-otlp-endpoint"),
        id = "TRACING_OTLP_ENDPOINT",
        value_name = "URL",
        help = "oltp endpoint",
        help_heading = "Node configuration",
        display_order = 26
    )]
    pub endpoint: Option<String>,
}

#[derive(clap::ValueEnum, Debug, Clone, Serialize)]
pub enum TracingType {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "otlp")]
    Otlp,
}

#[derive(Debug, Clone, Serialize)]
pub enum EnabledSystemServices {
    All,
    Some(Vec<String>),
    None,
}

// Parse either:
// - "all" to EnabledSystemServices::All
// - "none" to EnabledSystemServices::None
// - "service1,service2" to EnabledSystemServices::Some(vec!["service1", "service2"])
#[derive(Debug, Clone)]
struct EnabledSystemServicesValueParser;
impl clap::builder::TypedValueParser for EnabledSystemServicesValueParser {
    type Value = EnabledSystemServices;
    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let value = value
            .to_str()
            .ok_or_else(|| clap::Error::new(ErrorKind::InvalidUtf8))?;

        match value {
            "all" => Ok(EnabledSystemServices::All),
            "none" => Ok(EnabledSystemServices::None),
            _ => {
                let services = value.split(',').map(|s| s.to_string()).collect::<Vec<_>>();
                Ok(EnabledSystemServices::Some(services))
            }
        }
    }
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct SystemServicesArgs {
    // TODO: how to provide the list of available system services automatically
    #[arg(
    long,
    id = "SERVICES",
    help = "List of enabled system services. Can be: all, none or comma-separated list of services (serivce1,service2)",
    help_heading = "System services configuration",
    value_parser = EnabledSystemServicesValueParser
    )]
    enable_system_services: Option<EnabledSystemServices>,

    #[arg(
        long,
        id = "WALLET_KEY",
        help = "The private wallet key for signing transactions for joining deals",
        help_heading = "System services configuration"
    )]
    wallet_key: Option<String>,
}

impl Serialize for SystemServicesArgs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut struct_serializer = serializer.serialize_struct("SystemServicesConfig", 5)?;

        if let Some(enable_system_services) = &self.enable_system_services {
            match enable_system_services {
                EnabledSystemServices::All => {
                    let all = ServiceKey::all_values();
                    struct_serializer.serialize_field("enable", &all)?;
                }
                EnabledSystemServices::Some(services) => {
                    let services: Vec<ServiceKey> = services
                        .iter()
                        .map(|service| match ServiceKey::from_string(service) {
                            Some(service) => Ok(service),
                            None => Err(serde::ser::Error::custom(format!(
                                "unknown service: {}",
                                service
                            ))),
                        })
                        .collect::<Result<_, _>>()?;

                    struct_serializer.serialize_field("enable", &services)?;
                }
                EnabledSystemServices::None => {
                    struct_serializer.serialize_field::<Vec<ServiceKey>>("enable", &vec![])?;
                }
            }
        }
        if let Some(wallet_key) = &self.wallet_key {
            #[derive(Serialize)]
            struct DeciderConfig {
                wallet_key: String,
            }
            struct_serializer.serialize_field(
                "decider",
                &DeciderConfig {
                    wallet_key: wallet_key.clone(),
                },
            )?;
        }

        struct_serializer.end()
    }
}

#[derive(Parser, Debug, Serialize, Clone)]
pub(crate) struct DerivedArgs {
    #[arg(
        short,
        long,
        id = "TCP_PORT",
        help = "tcp port",
        help_heading = "Networking",
        display_order = 1
    )]
    tcp_port: Option<u16>,
    #[arg(
        short,
        long("ws-port"),
        id = "WEBSOCKET_PORT",
        help = "websocket port",
        help_heading = "Networking",
        display_order = 2
    )]
    websocket_port: Option<u16>,
    #[arg(
        short('s'),
        long,
        id = "HTTP_PORT",
        help = "http port",
        help_heading = "Networking",
        display_order = 3
    )]
    http_port: Option<u16>,
    #[arg(
        short('x'),
        long("external-ip"),
        id = "EXTERNAL_ADDR",
        help = "node external IP address to advertise to other peers",
        value_name = "IP",
        help_heading = "Networking",
        display_order = 4
    )]
    external_address: Option<String>,
    #[arg(
        short('z'),
        long("external-maddrs"),
        id = "EXTERNAL_MULTIADDRS",
        help = "external multiaddresses to advertize",
        value_name = "MULTIADDR",
        help_heading = "Networking",
        display_order = 5,
        action =  clap::ArgAction::Append,
        num_args = 1..
    )]
    external_multiaddresses: Option<Vec<String>>,
    #[arg(
        short('a'),
        long("allow-private-ips"),
        name = "ALLOW_PRIVATE_IPS",
        help = "allow private IP addresses from other nodes",
        help_heading = "Networking",
        display_order = 6,
        action =  clap::ArgAction::SetTrue
    )]
    allow_local_addresses: Option<bool>,
    #[arg(
        short('b'),
        long("bootstraps"),
        id = "BOOTSTRAP_NODE",
        help = "bootstrap nodes of the Fluence network",
        value_name = "MULTIADDR",
        help_heading = "Networking",
        display_order = 7,
        conflicts_with = "LOCAL",
        action =  clap::ArgAction::Append,
        num_args = 1..
    )]
    bootstrap_nodes: Option<Vec<String>>,
    #[arg(
        short('q'),
        long,
        id = "BOOTSTRAP_FREQ",
        help = "bootstrap kademlia each time N bootstraps (re)connect",
        value_name = "N",
        help_heading = "Networking",
        display_order = 8
    )]
    bootstrap_frequency: Option<usize>,
    #[arg(
        short('l'),
        long,
        id = "LOCAL",
        value_parser = clap::value_parser!(bool),
        help = "if passed, bootstrap nodes aren't used",
        help_heading = "Networking",
        display_order = 9,
        action =  clap::ArgAction::SetTrue
    )]
    local: Option<bool>,

    #[command(flatten)]
    root_key_pair: Option<RootKeyPairArgs>,

    #[arg(
        short('c'),
        long,
        id = "CONFIG_FILE",
        help_heading = "Node configuration",
        help = "TOML configuration file",
        value_name = "PATH",
        display_order = 15
    )]
    pub(crate) config: Option<PathBuf>,
    #[arg(
        short('d'),
        long,
        id = "CERTIFICATE_DIR",
        help_heading = "Node configuration",
        help = "certificate dir",
        value_name = "PATH",
        display_order = 16
    )]
    certificate_dir: Option<PathBuf>,
    #[arg(
        short('m'),
        long,
        id = "MANAGEMENT_PEER_ID",
        help_heading = "Node configuration",
        help = "PeerId of the node's administrator",
        value_name = "PEER ID",
        display_order = 17
    )]
    management_key: Option<String>,

    #[arg(
        short('u'),
        long,
        id = "BLUEPRINT_DIR",
        help_heading = "Services configuration",
        help = "directory containing blueprints and wasm modules",
        value_name = "PATH",
        display_order = 19
    )]
    blueprint_dir: Option<PathBuf>,
    #[arg(
        short('r'),
        long,
        id = "SERVICES_WORKDIR",
        help_heading = "Services configuration",
        help = "directory where all services will store their data",
        value_name = "PATH",
        display_order = 20
    )]
    services_workdir: Option<PathBuf>,
    #[arg(
        long("aqua-pool-size"),
        id = "AQUA_VM_POOL_SIZE",
        help_heading = "AIR configuration",
        help = "Number of AquaVM instances (particle script execution parallelism)",
        value_name = "NUM",
        display_order = 21
    )]
    aquavm_pool_size: Option<usize>,
    #[arg(
        long,
        value_parser = clap::value_parser!(bool),
        id = "PRINT_CONFIG",
        help = "Print applied config",
        help_heading = "Node configuration",
        display_order = 22,
        action = clap::ArgAction::SetTrue
    )]
    pub(crate) print_config: Option<bool>,
    #[arg(
        long,
        value_parser = clap::value_parser!(bool),
        id = "NO_BANNER",
        help = "Disable banner",
        help_heading = "Node configuration",
        display_order = 23,
        action = clap::ArgAction::SetTrue
    )]
    pub(crate) no_banner: Option<bool>,

    #[command(flatten)]
    system_services: Option<SystemServicesArgs>,

    #[command(flatten)]
    log: Option<LogArgs>,

    #[command(flatten)]
    tracing: Option<TracingArgs>,
}

impl Source for DerivedArgs {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let source_str =
            toml::to_string(&self).map_err(|e| config::ConfigError::Foreign(Box::new(e)))?;
        let result = toml::de::from_str(&source_str)
            .map_err(|e| config::ConfigError::Foreign(Box::new(e)))?;
        Ok(result)
    }
}
