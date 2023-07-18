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
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, Command, FromArgMatches};
use config::{Config, Environment, File, FileFormat, Source};
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

    pub log: Option<LogConfig>,
    pub tracing: Option<TracingConfig>,
    pub console: Option<ConsoleConfig>,
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
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Logfmt,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ConsoleConfig {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "enabled")]
    Enabled { bind: String },
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

    pub fn http_listen_addr(&self) -> Option<SocketAddr> {
        self.http_config
            .map(|config| SocketAddr::new(self.listen_config.listen_ip, config.http_port))
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

    let file_source = cli_config
        .config
        .clone()
        .or_else(|| std::env::var_os("FLUENCE_CONFIG").map(PathBuf::from))
        .map(|path| File::from(path).format(FileFormat::Toml))
        .unwrap_or(
            File::with_name("Config.toml")
                .required(false)
                .format(FileFormat::Toml),
        );

    let env_source = Environment::with_prefix("FLUENCE")
        .try_parsing(true)
        .prefix_separator("_")
        .separator("__")
        .list_separator(",")
        .with_list_parse_key("allowed_binaries")
        .with_list_parse_key("external_multiaddresses")
        .with_list_parse_key("bootstrap_nodes")
        .with_list_parse_key("listen_config.listen_multiaddrs")
        .with_list_parse_key("system_services.enable");

    println!("env_source: {:#?}", env_source.collect());

    let config = Config::builder()
        .add_source(file_source)
        .add_source(env_source)
        .add_source(cli_config)
        .build()?;

    let config: UnresolvedConfig = config.try_deserialize()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_keypair::KeyPair;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    use super::*;

    #[test]
    fn load_allowed_binaries_with_env() {
        temp_env::with_var(
            "FLUENCE_ALLOWED_BINARIES",
            Some("/bin/sh,/bin/bash"),
            || {
                let config = load_config_with_args(vec![], None).expect("Could not load config");
                assert_eq!(
                    config.node_config.allowed_binaries,
                    vec!("/bin/sh".to_string(), "/bin/bash".to_string())
                );
            },
        )
    }

    #[test]
    fn load_config_simple() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            root_key_pair.format = "ed25519"
            root_key_pair.secret_key = "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.value = "Ek6l5zgX9P74MHRiRzK/FN6ftQIOD3prYdMh87nRXlEEuRX1QrdQI87MBRdphoc0url0cY5ZO58evCoGXty1zw=="
            script_storage_max_failures = 10

            [root_weights]
            12D3KooWB9P1xmV3c7ZPpBemovbwCiRRTKd3Kq2jsVPQN4ZukDfy = 1

        "#).expect("Could not write in file");

        let path = file.path().display().to_string();
        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();
            let resolved_secret = encode_secret(&config);
            assert_eq!(config.node_config.script_storage_max_failures, 10);
            assert_eq!(config.node_config.allow_local_addresses, false);
            assert_eq!(
                resolved_secret,
                "/XKBs1ydmfWGiTbh+e49GYw+14LHtu+v5BMFDIzHpvo="
            );
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
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        let key_path = dir.path().join("secret_key.ed25519");
        let builtins_key_path = dir.path().join("builtins_secret_key.ed25519");
        write!(
            file,
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
        )
        .expect("Could not write in file");

        assert!(!key_path.exists());
        assert!(!builtins_key_path.exists());

        let path = file.path().display().to_string();
        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();
            let resolved_secret = encode_secret(&config);

            assert!(key_path.exists());
            assert!(builtins_key_path.exists());
            assert!(!resolved_secret.is_empty());
        });
    }

    #[test]
    fn load_empty_keypair() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        write!(
            file,
            r#"
            root_key_pair.generate_on_absence = true
            builtins_key_pair.generate_on_absence = true
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();
        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let _config = load_config_with_args(vec![], None).expect("Could not load config");
        })
    }

    #[test]
    fn load_empty_config() {
        let _config = load_config_with_args(vec![], None).expect("Could not load config");
    }

    #[test]
    fn load_base58_keypair() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        let root_key_path = dir.path().join("secret_key.ed25519");
        let builtins_key_path = dir.path().join("builtins_secret_key.ed25519");

        write!(
            file,
            r#"
            root_key_pair.format = "ed25519"
            root_key_pair.path = "{}"
            root_key_pair.generate_on_absence = false
            builtins_key_pair.format = "ed25519"
            builtins_key_pair.path = "{}"
            builtins_key_pair.generate_on_absence = false
            "#,
            root_key_path.to_string_lossy(),
            builtins_key_path.to_string_lossy()
        )
        .expect("Could not write in file");
        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
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
        });
    }

    #[test]
    fn load_base64_keypair() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        let root_key_path = dir.path().join("secret_key.ed25519");
        let builtins_key_path = dir.path().join("builtins_secret_key.ed25519");

        write!(
            file,
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
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let root_kp = KeyPair::generate_ed25519();
            let builtins_kp = KeyPair::generate_secp256k1();
            std::fs::write(&root_key_path, base64.encode(root_kp.to_vec())).unwrap();
            std::fs::write(&builtins_key_path, base64.encode(builtins_kp.to_vec())).unwrap();
            assert!(root_key_path.exists());
            assert!(builtins_key_path.exists());

            let _config = load_config_with_args(vec![], None).expect("Could not load config");
        });
    }

    #[test]
    fn load_base64_secret_key() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        let root_key_path = dir.path().join("secret_key.ed25519");
        let builtins_key_path = dir.path().join("builtins_secret_key.ed25519");

        write!(
            file,
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
        )
        .expect("Could not write in file");
        let root_kp = KeyPair::generate_ed25519();
        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
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
        });
    }

    #[test]
    fn load_base58_secret_key() {
        let dir = tempdir().expect("Could not create temp dir");
        let mut file = NamedTempFile::new_in(dir.path()).expect("Could not create temp file");

        let root_key_path = dir.path().join("secret_key.ed25519");
        let builtins_key_path = dir.path().join("builtins_secret_key.ed25519");

        write!(
            file,
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
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
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
        });
    }

    #[test]
    fn load_log_format_with_env() {
        temp_env::with_var("FLUENCE_LOG__FORMAT", Some("logfmt"), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let log_fmt = config.log.map(|x| x.format);
            assert_eq!(log_fmt, Some(LogFormat::Logfmt));
        });
    }

    #[test]
    fn load_log_format_with_args() {
        let args = vec![
            OsString::from("nox"),
            OsString::from("--log-format"),
            OsString::from("logfmt"),
        ];
        let config = load_config_with_args(args, None).expect("Could not load config");
        let log_fmt = config.log.map(|x| x.format);
        assert_eq!(log_fmt, Some(LogFormat::Logfmt));
    }

    #[test]
    fn load_log_format_with_file() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            [log]
            format = "logfmt"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let log_fmt = config.log.map(|x| x.format);
            assert_eq!(log_fmt, Some(LogFormat::Logfmt));
        });
    }

    #[test]
    fn load_allowed_binaries_with_file() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            allowed_binaries = ["/bin/sh"]
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.node_config.allowed_binaries,
                vec!("/bin/sh".to_string())
            );
        });
    }

    #[test]
    fn load_tracing_disabled_with_file() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            [tracing]
            type = "disabled"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(config.tracing, Some(TracingConfig::Disabled));
        });
    }

    #[test]
    fn load_tracing_otlp_with_file() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            [tracing]
            type = "otlp"
            endpoint = "test"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: "test".to_string()
                })
            );
        });
    }

    #[test]
    fn load_tracing_disabled_with_env() {
        temp_env::with_var("FLUENCE_TRACING__TYPE", Some("disabled"), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(config.tracing, Some(TracingConfig::Disabled));
        });
    }

    #[test]
    fn load_tracing_otlp_with_env() {
        temp_env::with_vars(
            [
                ("FLUENCE_TRACING__TYPE", Some("otlp")),
                ("FLUENCE_TRACING__ENDPOINT", Some("test")),
            ],
            || {
                let config = load_config_with_args(vec![], None).expect("Could not load config");
                assert_eq!(
                    config.tracing,
                    Some(TracingConfig::Otlp {
                        endpoint: "test".to_string()
                    })
                );
            },
        );
    }

    #[test]
    fn load_tracing_disabled_with_args() {
        temp_env::with_vars(
            [
                ("FLUENCE_TRACING__TYPE", Some("otlp")),
                ("FLUENCE_TRACING__ENDPOINT", Some("test")),
            ],
            || {
                let args = vec![
                    OsString::from("nox"),
                    OsString::from("--tracing-type"),
                    OsString::from("disabled"),
                ];
                let config = load_config_with_args(args, None).expect("Could not load config");
                assert_eq!(config.tracing, Some(TracingConfig::Disabled));
            },
        );
    }

    #[test]
    fn load_tracing_otlp_with_args() {
        temp_env::with_var("FLUENCE_TRACING__TYPE", Some("disabled"), || {
            let args = vec![
                OsString::from("nox"),
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
        });
    }
}
