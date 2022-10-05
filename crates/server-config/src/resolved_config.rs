/*
 * Copyright 2021 Fluence Labs Limited
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

use crate::defaults::default_config_path;
use crate::dir_config::{ResolvedDirConfig, UnresolvedDirConfig};
use crate::node_config::NodeConfig;

use fs_utils::to_abs_path;

use clap::{ArgMatches, Values};
use eyre::{eyre, ContextCompat, WrapErr};
use libp2p::core::{multiaddr::Protocol, Multiaddr};
use serde::Deserialize;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

pub const WEBSOCKET_PORT: &str = "websocket_port";
pub const TCP_PORT: &str = "tcp_port";
pub const ROOT_KEY_PAIR: &str = "root_key_pair";
pub const ROOT_KEY_PAIR_VALUE: &str = "value";
pub const ROOT_KEY_FORMAT: &str = "format";
pub const ROOT_KEY_PAIR_PATH: &str = "path";
pub const ROOT_KEY_PAIR_GENERATE: &str = "generate_on_absence";
pub const BOOTSTRAP_NODE: &str = "bootstrap_nodes";
pub const BOOTSTRAP_FREQ: &str = "bootstrap_frequency";
pub const EXTERNAL_ADDR: &str = "external_address";
pub const EXTERNAL_MULTIADDRS: &str = "external_multiaddresses";
pub const CERTIFICATE_DIR: &str = "certificate_dir";
pub const CONFIG_FILE: &str = "config_file";
pub const SERVICE_ENVS: &str = "service_envs";
pub const BLUEPRINT_DIR: &str = "blueprint_dir";
pub const MANAGEMENT_PEER_ID: &str = "management_peer_id";
pub const SERVICES_WORKDIR: &str = "services_workdir";
pub const LOCAL: &str = "local";
pub const ALLOW_PRIVATE_IPS: &str = "allow_local_addresses";
pub const METRICS_PORT: &str = "metrics_port";
pub const AQUA_VM_POOL_SIZE: &str = "aquavm_pool_size";
pub const SECRET_KEY: &str = "secret_key";

const ARGS: &[&str] = &[
    WEBSOCKET_PORT,
    TCP_PORT,
    ROOT_KEY_PAIR_VALUE,
    ROOT_KEY_PAIR_GENERATE,
    ROOT_KEY_FORMAT,
    ROOT_KEY_PAIR_PATH,
    BOOTSTRAP_NODE,
    BOOTSTRAP_FREQ,
    EXTERNAL_ADDR,
    EXTERNAL_MULTIADDRS,
    CERTIFICATE_DIR,
    CONFIG_FILE,
    SERVICE_ENVS,
    BLUEPRINT_DIR,
    MANAGEMENT_PEER_ID,
    ALLOW_PRIVATE_IPS,
    METRICS_PORT,
    AQUA_VM_POOL_SIZE,
    SECRET_KEY,
];

#[derive(Clone, Deserialize, Debug)]
pub struct UnresolvedConfig {
    #[serde(flatten)]
    dir_config: UnresolvedDirConfig,
    #[serde(flatten)]
    node_config: NodeConfig,
}

impl UnresolvedConfig {
    pub fn resolve(self) -> ResolvedConfig {
        ResolvedConfig {
            dir_config: self.dir_config.resolve(),
            node_config: self.node_config,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub dir_config: ResolvedDirConfig,
    pub node_config: NodeConfig,
}

impl Deref for ResolvedConfig {
    type Target = NodeConfig;

    fn deref(&self) -> &Self::Target {
        &self.node_config
    }
}

impl DerefMut for ResolvedConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node_config
    }
}

impl ResolvedConfig {
    pub fn external_addresses(&self) -> Vec<Multiaddr> {
        let mut addrs = if let Some(external_address) = self.external_address {
            let external_tcp = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(self.listen_config.tcp_port));
                maddr
            };

            let external_ws = {
                let mut maddr = Multiaddr::from(external_address);
                maddr.push(Protocol::Tcp(self.listen_config.websocket_port));
                maddr.push(Protocol::Ws("/".into()));
                maddr
            };

            vec![external_tcp, external_ws]
        } else {
            vec![]
        };

        addrs.extend(self.external_multiaddresses.iter().cloned());

        addrs
    }

    pub fn metrics_listen_addr(&self) -> SocketAddr {
        SocketAddr::new(
            self.listen_config.listen_ip,
            self.metrics_config.metrics_port,
        )
    }

    pub fn listen_multiaddrs(&self) -> Vec<Multiaddr> {
        let config = &self.listen_config;

        let mut tcp = Multiaddr::from(config.listen_ip);
        tcp.push(Protocol::Tcp(config.tcp_port));

        let mut ws = Multiaddr::from(config.listen_ip);
        ws.push(Protocol::Tcp(config.websocket_port));
        ws.push(Protocol::Ws("/".into()));

        vec![tcp, ws]
    }
}

/// Take all command line arguments, and insert them into config appropriately
fn insert_args_to_config(
    arguments: &ArgMatches,
    config: &mut toml::value::Table,
) -> eyre::Result<()> {
    use toml::Value::*;

    fn single(mut value: Values<'_>) -> eyre::Result<&str> {
        value.next().wrap_err("no more arguments")
    }

    fn multiple(value: Values<'_>) -> impl Iterator<Item = toml::Value> + '_ {
        value.map(|s| String(s.into()))
    }

    // Check each possible command line argument
    for &k in ARGS {
        let arg = match arguments.values_of(k) {
            Some(arg) => arg,
            None => continue,
        };

        let result: eyre::Result<()> = try {
            let k = k.to_string();
            // Convert value to a type of the corresponding field in `FluenceConfig` and insert into config
            match k.as_str() {
                WEBSOCKET_PORT | TCP_PORT | METRICS_PORT | AQUA_VM_POOL_SIZE => {
                    config.insert(k, Integer(single(arg)?.parse()?))
                }
                BOOTSTRAP_NODE | SERVICE_ENVS | EXTERNAL_MULTIADDRS => {
                    config.insert(k, Array(multiple(arg).collect()))
                }
                ALLOW_PRIVATE_IPS => config.insert(k, Boolean(true)),
                ROOT_KEY_PAIR_PATH
                | ROOT_KEY_PAIR_VALUE
                | ROOT_KEY_FORMAT
                | ROOT_KEY_PAIR_GENERATE
                | SECRET_KEY => {
                    let value = if k == ROOT_KEY_PAIR_GENERATE {
                        match single(arg)? {
                            "true" => Boolean(true),
                            "false" => Boolean(false),
                            other => return Err(eyre!("Invalid value for ROOT_KEY_PAIR_GENERATE. Must be true or false, was {}", other)), 
                        }
                    } else {
                        String(single(arg)?.into())
                    };

                    let mut key_pair_config = config
                        .remove(ROOT_KEY_PAIR)
                        .unwrap_or(toml::Value::Table(<_>::default()));
                    key_pair_config
                        .as_table_mut()
                        .expect("root key pair must be a toml table")
                        .insert(k, value);

                    config.insert(ROOT_KEY_PAIR.to_string(), key_pair_config)
                }
                _ => config.insert(k, String(single(arg)?.into())),
            };
        };
        result.context(format!("error processing argument '{}'", k))?
    }

    if let Some(key_pair_config) = config.get_mut(ROOT_KEY_PAIR).and_then(|v| v.as_table_mut()) {
        // 'secret key' overrides 'key pair path' and 'key pair value'
        if key_pair_config.contains_key(SECRET_KEY) {
            key_pair_config.remove(ROOT_KEY_PAIR_PATH);
            key_pair_config.remove(ROOT_KEY_PAIR_VALUE);
        }

        // 'key pair value' overrides 'key pair path'
        if key_pair_config.contains_key(ROOT_KEY_PAIR_VALUE) {
            key_pair_config.remove(ROOT_KEY_PAIR_PATH);
        }
    }

    Ok(())
}

// loads config from arguments and a config file
// TODO: avoid depending on ArgMatches
pub fn load_config(arguments: ArgMatches) -> eyre::Result<ResolvedConfig> {
    let config_file = arguments.value_of(CONFIG_FILE).map(Into::into);
    let config_file = config_file.unwrap_or(default_config_path());

    let config_bytes = if config_file.is_file() {
        let config_file = to_abs_path(config_file);

        log::info!("Loading config from {:?}", config_file);

        std::fs::read(&config_file)
            .wrap_err_with(|| format!("Failed reading config {:?}", config_file))?
    } else {
        log::info!("Config wasn't found, using default settings");
        Vec::default()
    };

    let config = deserialize_config(&arguments, &config_bytes)
        .wrap_err(eyre!("config deserialization failed"))?;

    config.dir_config.create_dirs()?;

    Ok(config)
}

pub fn deserialize_config(arguments: &ArgMatches, content: &[u8]) -> eyre::Result<ResolvedConfig> {
    let mut config: toml::value::Table =
        toml::from_slice(content).wrap_err("deserializing config")?;

    insert_args_to_config(arguments, &mut config)?;

    let config = toml::value::Value::Table(config);
    let mut config = UnresolvedConfig::deserialize(config)?.resolve();

    if arguments.is_present(LOCAL) {
        // if --local is passed, clear bootstrap nodes
        config.bootstrap_nodes = vec![];
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use crate::BootstrapConfig;

    use super::*;
    use fs_utils::make_tmp_dir;

    #[test]
    fn parse_config() {
        let config = r#"
            root_key_pair.format = "ed25519"
            root_key_pair.value = "Ek6l5zgX9P74MHRiRzK/FN6ftQIOD3prYdMh87nRXlEEuRX1QrdQI87MBRdphoc0url0cY5ZO58evCoGXty1zw=="
            root_key_pair.secret_key = "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.value = "Ek6l5zgX9P74MHRiRzK/FN6ftQIOD3prYdMh87nRXlEEuRX1QrdQI87MBRdphoc0url0cY5ZO58evCoGXty1zw=="
            builtins_key_pair.secret_key = "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            avm_base_dir = "/stepper"
            script_storage_max_failures = 10
            
            [root_weights]
            12D3KooWB9P1xmV3c7ZPpBemovbwCiRRTKd3Kq2jsVPQN4ZukDfy = 1
            
        "#;

        let config =
            deserialize_config(&<_>::default(), config.as_bytes()).expect("deserialize config");

        assert_eq!(config.node_config.script_storage_max_failures, 10);
    }

    #[test]
    fn parse_path_keypair() {
        let key_path = make_tmp_dir().join("secret_key.ed25519");
        let builtins_key_path = make_tmp_dir().join("builtins_secret_key.ed25519");
        let config = format!(
            r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = true
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = true
            "#,
            key_path.to_string_lossy(),
            builtins_key_path.to_string_lossy(),
        );

        assert!(!key_path.exists());
        assert!(!builtins_key_path.exists());
        deserialize_config(&<_>::default(), config.as_bytes()).expect("deserialize config");
        assert!(key_path.exists());
        assert!(builtins_key_path.exists());
    }

    #[test]
    fn parse_empty_keypair() {
        let config = r#"
            root_key_pair.generate_on_absence = true
            builtins_key_pair.generate_on_absence = true
            "#;
        deserialize_config(&<_>::default(), config.as_bytes()).expect("deserialize config");
    }

    #[test]
    fn parse_empty_config() {
        deserialize_config(&<_>::default(), &[]).expect("deserialize config");
    }

    #[test]
    fn duration() {
        let bs_config = BootstrapConfig::default();
        let s = toml::to_string(&bs_config).expect("serialize");
        println!("{}", s)
    }
}
