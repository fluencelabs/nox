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

use std::ffi::OsString;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use clap::{Args, Command, FromArgMatches};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use libp2p::core::{multiaddr::Protocol, Multiaddr};
use serde::{Deserialize, Serialize};

use crate::args;
use crate::dir_config::{ResolvedDirConfig, UnresolvedDirConfig};
use crate::node_config::{NodeConfig, UnresolvedNodeConfig};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UnresolvedConfig {
    #[serde(flatten)]
    dir_config: UnresolvedDirConfig,
    #[serde(flatten)]
    node_config: UnresolvedNodeConfig,
    #[serde(default)]
    pub log: Option<LogConfig>,
    #[serde(default)]
    pub tracing: Option<TracingConfig>,

    pub no_banner: Option<bool>,

    pub print_config: Option<bool>,
}

impl UnresolvedConfig {
    pub fn resolve(self) -> eyre::Result<ResolvedConfig> {
        Ok(ResolvedConfig {
            dir_config: self.dir_config.resolve()?,
            node_config: self.node_config.resolve()?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogConfig {
    pub format: LogFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum LogFormat {
    #[serde(rename = "logfmt")]
    Logfmt,
    #[serde(rename = "default")]
    Default,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "logfmt" => Ok(LogFormat::Logfmt),
            "default" => Ok(LogFormat::Default),
            _ => Err("Unsupported log format".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TracingConfig {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "otlp")]
    Otlp { endpoint: String },
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

pub struct ConfigData {
    pub binary_name: String,
    pub version: String,
    pub authors: String,
    pub description: String,
}

pub fn load_config(data: Option<ConfigData>) -> eyre::Result<UnresolvedConfig> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    load_config_with_args(raw_args, data)
}

pub fn load_config_with_args(
    raw_args: Vec<OsString>,
    data: Option<ConfigData>,
) -> eyre::Result<UnresolvedConfig> {
    let command = Command::new("Fluence peer");
    let command = if let Some(data) = data {
        command
            .version(&data.version)
            .author(&data.authors)
            .about(data.description)
            .override_usage(format!("{} [FLAGS] [OPTIONS]", data.binary_name))
    } else {
        command
    };

    let raw_cli_config = args::DerivedArgs::augment_args(command);
    let matches = raw_cli_config.get_matches_from(raw_args);
    let cli_config = args::DerivedArgs::from_arg_matches(&matches)?;

    let config_builder: Figment = Figment::new();
    let config_builder = if let Some(config_path) = cli_config.config.clone() {
        let extension = config_path.extension();
        if let Some(extension) = extension {
            match extension.to_str() {
                Some("toml") => config_builder.merge(Toml::file(config_path)),
                _ => config_builder,
            }
        } else {
            config_builder
        }
    } else {
        config_builder.merge(Toml::file(Env::var_or("FLUENCE_CONFIG", "Config.toml")))
    };

    let config_builder = config_builder
        .merge(Env::prefixed("FLUENCE_").split("__"))
        .merge(cli_config);

    let config: UnresolvedConfig = config_builder.extract()?;

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

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();
            let resolved_secret = encode_secret(&config);
            assert_eq!(config.node_config.script_storage_max_failures, 10);
            assert_eq!(
                resolved_secret,
                "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            );
            Ok(())
        });
    }

    fn encode_secret(config: &ResolvedConfig) -> String {
        match config.root_key_pair.clone() {
            KeyPair::Ed25519(x) => base64.encode(x.secret().0),
            KeyPair::Rsa(_) => "".to_string(),
            KeyPair::Secp256k1(_) => "".to_string(),
        }
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
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();
            let resolved_secret = encode_secret(&config);

            assert!(key_path.exists());
            assert!(builtins_key_path.exists());
            assert!(!resolved_secret.is_empty());
            Ok(())
        });
    }

    #[test]
    fn load_empty_keypair() {
        Jail::expect_with(|jail| {
            let _file = jail.create_file(
                "Config.toml",
                r#"
            root_key_pair.generate_on_absence = true
            builtins_key_pair.generate_on_absence = true
            "#,
            )?;
            let _config = load_config_with_args(vec![], None).expect("Could not load config");
            Ok(())
        });
    }

    #[test]
    fn load_empty_config() {
        let _config = load_config_with_args(vec![], None).expect("Could not load config");
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

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();

            let resolved_secret = encode_secret(&config);
            assert_eq!(resolved_secret, base64.encode(root_kp.secret().unwrap()));

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

            let _config = load_config_with_args(vec![], None).expect("Could not load config");

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

            let _config = load_config_with_args(vec![], None).expect("Could not load config");

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

            let _config = load_config_with_args(vec![], None).expect("Could not load config");

            Ok(())
        });
    }

    #[test]
    fn load_log_format_with_env() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_LOG__FORMAT", "logfmt");

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let log_fmt = config.log.map(|x| x.format);
            assert_eq!(log_fmt, Some(LogFormat::Logfmt));
            Ok(())
        });
    }

    #[test]
    fn load_log_format_with_args() {
        Jail::expect_with(|_jail| {
            let args = vec![
                OsString::from("particle-node"),
                OsString::from("--log-format"),
                OsString::from("logfmt"),
            ];
            let config = load_config_with_args(args, None).expect("Could not load config");
            let log_fmt = config.log.map(|x| x.format);
            assert_eq!(log_fmt, Some(LogFormat::Logfmt));
            Ok(())
        });
    }

    #[test]
    fn load_log_format_with_file() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "Config.toml",
                r#"
            [log]
            format = "logfmt"
            "#,
            )?;

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let log_fmt = config.log.map(|x| x.format);
            assert_eq!(log_fmt, Some(LogFormat::Logfmt));
            Ok(())
        })
    }

    #[test]
    fn load_allowed_binaries_with_file() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "Config.toml",
                r#"
            allowed_binaries = ["/bin/sh"]
            "#,
            )?;

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.node_config.allowed_binaries,
                vec!("/bin/sh".to_string())
            );

            Ok(())
        });
    }

    #[test]
    fn load_allowed_binaries_with_env() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_ALLOWED_BINARIES", "[\"/bin/sh\"]");

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.node_config.allowed_binaries,
                vec!("/bin/sh".to_string())
            );

            Ok(())
        });
    }

    #[test]
    fn load_tracing_disabled_with_file() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "Config.toml",
                r#"
            [tracing]
            type = "disabled"
            "#,
            )?;

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(config.tracing, Some(TracingConfig::Disabled));
            Ok(())
        })
    }

    #[test]
    fn load_tracing_otlp_with_file() {
        Jail::expect_with(|jail| {
            jail.create_file(
                "Config.toml",
                r#"
            [tracing]
            type = "otlp"
            endpoint = "test"
            "#,
            )?;

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: "test".to_string()
                })
            );
            Ok(())
        })
    }

    #[test]
    fn load_tracing_disabled_with_env() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_TRACING__TYPE", "disabled");

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(config.tracing, Some(TracingConfig::Disabled));
            Ok(())
        });
    }

    #[test]
    fn load_tracing_otlp_with_env() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_TRACING__TYPE", "otlp");
            jail.set_env("FLUENCE_TRACING__ENDPOINT", "test");

            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: "test".to_string()
                })
            );
            Ok(())
        });
    }

    #[test]
    fn load_tracing_disabled_with_args() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_TRACING__TYPE", "otlp");
            jail.set_env("FLUENCE_TRACING__ENDPOINT", "test");

            let args = vec![
                OsString::from("particle-node"),
                OsString::from("--tracing-type"),
                OsString::from("disabled"),
            ];
            let config = load_config_with_args(args, None).expect("Could not load config");
            assert_eq!(config.tracing, Some(TracingConfig::Disabled));
            Ok(())
        });
    }

    #[test]
    fn load_tracing_otlp_with_args() {
        Jail::expect_with(|jail| {
            jail.set_env("FLUENCE_TRACING__TYPE", "disabled");

            let args = vec![
                OsString::from("particle-node"),
                OsString::from("--tracing-type"),
                OsString::from("otlp"),
                OsString::from("--tracing-otlp-endpoint"),
                OsString::from("test"),
            ];
            let config = load_config_with_args(args, None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: "test".to_string()
                })
            );
            Ok(())
        });
    }
}
