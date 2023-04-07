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

use clap::{Args, Parser};
use figment::error::Kind::InvalidType;
use figment::value::{Dict, Map, Value};
use figment::{Error, Metadata, Profile};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct RootKeyPairArgs {
    #[arg(
        short('k'),
        long,
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
        long,
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
    long,
    value_parser(["ed25519", "secp256k1", "rsa"]),
    id = "ROOT_KEY_FORMAT",
    help_heading = "Node keypair",
    display_order = 12,
    )]
    format: Option<String>,

    #[arg(
    short('g'),
    long,
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
        id = "METRICS_PORT",
        help = "open metrics port",
        help_heading = "Networking",
        display_order = 3
    )]
    metrics_port: Option<u16>,
    #[arg(
        short('x'),
        long,
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
        long,
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
        long,
        id = "AQUA_VM_POOL_SIZE",
        help_heading = "AIR configuration",
        help = "Number of AquaVM instances (particle script execution parallelism)",
        value_name = "NUM",
        display_order = 21
    )]
    aqua_pool_size: Option<PathBuf>,
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
}

impl figment::Provider for DerivedArgs {
    fn metadata(&self) -> Metadata {
        Metadata::named("Args")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let value = Value::serialize(self)?;

        let error = InvalidType(value.to_actual(), "map".into());
        let dict = value
            .into_dict()
            .map(|dict| {
                let a = dict.into_iter().filter_map(|(key, value)| match value {
                    Value::Empty(_, _) => None,
                    value => Some((key, value)),
                });
                a.collect::<Dict>()
            })
            .ok_or(error)?;
        Ok(Map::from([(Profile::Default, dict)]))
    }
}
