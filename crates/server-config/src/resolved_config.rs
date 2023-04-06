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

use clap::{Args, Command, FromArgMatches};
use std::ffi::OsString;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use crate::args;
use figment::{
    providers::{Env, Format, Json, Toml},
    Figment,
};
use libp2p::core::{multiaddr::Protocol, Multiaddr};
use serde::{Deserialize, Serialize};

use crate::dir_config::{ResolvedDirConfig, UnresolvedDirConfig};
use crate::node_config::NodeConfig;

#[derive(Clone, Deserialize, Debug)]
pub struct UnresolvedConfig {
    #[serde(flatten)]
    dir_config: UnresolvedDirConfig,
    #[serde(flatten)]
    node_config: NodeConfig,
    #[serde(default)]
    local: Option<bool>,
}

impl UnresolvedConfig {
    pub fn resolve(mut self) -> eyre::Result<ResolvedConfig> {
        let bootstrap_nodes = match self.local {
            Some(true) => vec![],
            _ => self.node_config.bootstrap_nodes,
        };

        self.node_config.bootstrap_nodes = bootstrap_nodes;

        Ok(ResolvedConfig {
            dir_config: self.dir_config.resolve()?,
            node_config: self.node_config,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

pub struct ConfigData {
    pub version: String,
    pub authors: String,
    pub description: String,
}

pub fn load_config(data: Option<ConfigData>) -> eyre::Result<ResolvedConfig> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    resolve_config(raw_args, data)
}

pub fn resolve_config(
    raw_args: Vec<OsString>,
    data: Option<ConfigData>,
) -> eyre::Result<ResolvedConfig> {
    let command = Command::new("Fluence node").override_usage(r#"particle-node [FLAGS] [OPTIONS]"#);
    let command = if let Some(data) = data {
        command
            .version(&data.version)
            .author(&data.authors)
            .about(data.description)
    } else {
        command
    };

    let raw_cli_config = args::DerivedArgs::augment_args(command);
    let matches = raw_cli_config.get_matches_from(raw_args);
    let cli_config = args::DerivedArgs::from_arg_matches(&matches)
        .map_err(|err| err.exit())
        .unwrap();

    let config_builder: Figment = Figment::new();
    let config_builder = if let Some(config_path) = cli_config.config.clone() {
        let extension = config_path.extension();
        if let Some(extension) = extension {
            match extension.to_str() {
                Some("toml") => config_builder.merge(Toml::file(config_path)),
                Some("json") => config_builder.merge(Json::file(config_path)),
                _ => config_builder,
            }
        } else {
            config_builder
        }
    } else {
        config_builder
            .merge(Toml::file(Env::var_or("FLUENCE_CONFIG", "Config.toml")))
            .merge(Json::file(Env::var_or("FLUENCE_CONFIG", "Config.json")))
    };

    let config_builder = config_builder
        .merge(Env::prefixed("FLUENCE_"))
        .merge(cli_config.clone());

    let config: UnresolvedConfig = config_builder.extract()?;

    let config = config.resolve()?;

    if let Some(true) = cli_config.print_config {
        log::info!("Loaded config: {:#?}", config);
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use figment::Jail;
    use fluence_keypair::KeyPair;

    use super::*;

    #[test]
    fn load_config_simple() {
        Jail::expect_with(|jail| {
            jail.create_file("Config.toml",
                             r#"
            root_key_pair.format = "ed25519"
            root_key_pair.secret_key = "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.value = "Ek6l5zgX9P74MHRiRzK/FN6ftQIOD3prYdMh87nRXlEEuRX1QrdQI87MBRdphoc0url0cY5ZO58evCoGXty1zw=="
            avm_base_dir = "{}"
            script_storage_max_failures = 10

            [root_weights]
            12D3KooWB9P1xmV3c7ZPpBemovbwCiRRTKd3Kq2jsVPQN4ZukDfy = 1

        "#)?;

            let config = resolve_config(vec![], None).expect("Could not load config");
            assert_eq!(config.node_config.script_storage_max_failures, 10);
            Ok(())
        });
    }

    #[test]
    fn load_path_keypair_generate() {
        Jail::expect_with(|jail| {
            let key_path = jail.directory().join("secret_key.ed25519");
            let builtins_key_path = jail.directory().join("builtins_secret_key.ed25519");
            jail.create_file(
                "Config.toml",
                &format!(
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
                ),
            )?;

            assert!(!key_path.exists());
            assert!(!builtins_key_path.exists());
            let _config = resolve_config(vec![], None).expect("Could not load config");
            assert!(key_path.exists());
            assert!(builtins_key_path.exists());
            Ok(())
        });
    }

    #[test]
    fn load_empty_keypair() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "Config.toml",
                r#"
            root_key_pair.generate_on_absence = true
            builtins_key_pair.generate_on_absence = true
            "#,
            )?;
            let _config = resolve_config(vec![], None).expect("Could not load config");
            Ok(())
        });
    }

    #[test]
    fn load_empty_config() {
        let _config = resolve_config(vec![], None).expect("Could not load config");
    }

    #[test]
    fn load_base58_keypair() {
        Jail::expect_with(|jail| {
            let root_key_path = jail.directory().join("secret_key.ed25519");
            let builtins_key_path = jail.directory().join("builtins_secret_key.ed25519");

            jail.create_file(
                "Config.toml",
                &format!(
                    r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = false
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = false
            "#,
                    root_key_path.to_string_lossy(),
                    builtins_key_path.to_string_lossy(),
                ),
            )?;

            let root_kp = KeyPair::generate_ed25519();
            let builtins_kp = KeyPair::generate_secp256k1();
            std::fs::write(&root_key_path, bs58::encode(root_kp.to_vec()).into_vec()).unwrap();
            std::fs::write(
                &builtins_key_path,
                bs58::encode(builtins_kp.to_vec()).into_vec(),
            )
            .unwrap();
            assert!(root_key_path.exists());
            assert!(builtins_key_path.exists());

            let _config = resolve_config(vec![], None).expect("Could not load config");

            Ok(())
        });
    }

    #[test]
    fn load_base64_keypair() {
        Jail::expect_with(|jail| {
            let root_key_path = jail.directory().join("secret_key.ed25519");
            let builtins_key_path = jail.directory().join("builtins_secret_key.ed25519");

            jail.create_file(
                "Config.toml",
                &format!(
                    r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = false
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = false
            "#,
                    root_key_path.to_string_lossy(),
                    builtins_key_path.to_string_lossy(),
                ),
            )?;

            let root_kp = KeyPair::generate_ed25519();
            let builtins_kp = KeyPair::generate_secp256k1();
            std::fs::write(&root_key_path, base64.encode(root_kp.to_vec())).unwrap();
            std::fs::write(&builtins_key_path, base64.encode(builtins_kp.to_vec())).unwrap();
            assert!(root_key_path.exists());
            assert!(builtins_key_path.exists());

            let _config = resolve_config(vec![], None).expect("Could not load config");

            Ok(())
        });
    }

    #[test]
    fn load_base64_secret_key() {
        Jail::expect_with(|jail| {
            let root_key_path = jail.directory().join("secret_key.ed25519");
            let builtins_key_path = jail.directory().join("builtins_secret_key.ed25519");

            jail.create_file(
                "Config.toml",
                &format!(
                    r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = false
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = false
            "#,
                    root_key_path.to_string_lossy(),
                    builtins_key_path.to_string_lossy(),
                ),
            )?;

            let root_kp = KeyPair::generate_ed25519();
            let builtins_kp = KeyPair::generate_secp256k1();
            std::fs::write(&root_key_path, base64.encode(&root_kp.secret().unwrap())).unwrap();
            std::fs::write(
                &builtins_key_path,
                base64.encode(&builtins_kp.secret().unwrap()),
            )
            .unwrap();
            assert!(root_key_path.exists());
            assert!(builtins_key_path.exists());

            let _config = resolve_config(vec![], None).expect("Could not load config");

            Ok(())
        });
    }

    #[test]
    fn load_base58_secret_key() {
        Jail::expect_with(|jail| {
            let root_key_path = jail.directory().join("secret_key.ed25519");
            let builtins_key_path = jail.directory().join("builtins_secret_key.ed25519");

            jail.create_file(
                "Config.toml",
                &format!(
                    r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = false
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = false
            "#,
                    root_key_path.to_string_lossy(),
                    builtins_key_path.to_string_lossy(),
                ),
            )?;

            let root_kp = KeyPair::generate_ed25519();
            let builtins_kp = KeyPair::generate_secp256k1();
            std::fs::write(
                &root_key_path,
                bs58::encode(root_kp.secret().unwrap().to_vec()).into_vec(),
            )
            .unwrap();
            std::fs::write(
                &builtins_key_path,
                bs58::encode(builtins_kp.secret().unwrap().to_vec()).into_vec(),
            )
            .unwrap();
            assert!(root_key_path.exists());
            assert!(builtins_key_path.exists());

            let _config = resolve_config(vec![], None).expect("Could not load config");

            Ok(())
        });
    }
}
