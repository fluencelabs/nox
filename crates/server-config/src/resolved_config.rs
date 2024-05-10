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

use clap::{Args, Command, FromArgMatches};
use config::{Config, Environment, File, FileFormat, FileSourceFile};
use libp2p::core::{multiaddr::Protocol, Multiaddr};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::args;
use crate::args::DerivedArgs;
use crate::dir_config::{ResolvedDirConfig, UnresolvedDirConfig};
use crate::node_config::{NodeConfig, UnresolvedNodeConfig};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UnresolvedConfig {
    #[serde(flatten)]
    dir_config: UnresolvedDirConfig,

    #[serde(flatten)]
    pub node_config: UnresolvedNodeConfig,

    pub tracing: Option<TracingConfig>,

    pub no_banner: Option<bool>,

    pub print_config: Option<bool>,
}

impl UnresolvedConfig {
    pub fn resolve(self) -> eyre::Result<ResolvedConfig> {
        let dir_config = self.dir_config.resolve()?;
        let node_config = self.node_config.resolve(&dir_config.persistent_base_dir)?;

        Ok(ResolvedConfig {
            dir_config,
            node_config,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TracingConfig {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "otlp")]
    Otlp {
        endpoint: Url,
        sample_ratio: Option<f64>,
    },
}

#[derive(Clone, Debug, Serialize)]
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

/// Hierarchically loads the configuration using args and envs.
/// The source order is:
///  - Load and parse Config.toml from cwd, if exists
///  - Load and parse files provided by FLUENCE_CONFIG env var
///  - Load and parse files provided by --config arg
///  - Load config values from env vars
///  - Load config values from args (throw error on conflicts with env vars)
/// On each stage the values override the previous ones.
///
/// # Arguments
///
/// - `data`: Optional `ConfigData` to customize the configuration.
///
/// # Returns
///
/// Returns a Result containing the unresolved configuration or an Eyre error.
///
pub fn load_config(data: Option<ConfigData>) -> eyre::Result<UnresolvedConfig> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    load_config_with_args(raw_args, data)
}

pub fn load_config_with_args(
    raw_args: Vec<OsString>,
    data: Option<ConfigData>,
) -> eyre::Result<UnresolvedConfig> {
    let arg_source = process_args(raw_args, data)?;

    let arg_config_sources: Vec<File<FileSourceFile, FileFormat>> = arg_source
        .configs
        .iter()
        .flat_map(|paths| {
            paths
                .iter()
                .map(|path| File::from(path.clone()).format(FileFormat::Toml))
                .collect::<Vec<File<FileSourceFile, FileFormat>>>()
        })
        .collect();

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

    let env_config_sources: Vec<File<FileSourceFile, FileFormat>> =
        std::env::var_os("FLUENCE_CONFIG")
            .and_then(|str| str.into_string().ok())
            .map(|str| {
                str.trim()
                    .split(',')
                    .map(PathBuf::from)
                    .map(|path| File::from(path.clone()).format(FileFormat::Toml))
                    .collect()
            })
            .unwrap_or_default();

    let mut config_builder = Config::builder().add_source(
        File::with_name("Config.toml")
            .required(false)
            .format(FileFormat::Toml),
    );

    for source in env_config_sources {
        config_builder = config_builder.add_source(source)
    }

    for source in arg_config_sources {
        config_builder = config_builder.add_source(source)
    }
    config_builder = config_builder.add_source(env_source).add_source(arg_source);
    let config = config_builder.build()?;

    let config: UnresolvedConfig = config.try_deserialize()?;

    Ok(config)
}

fn process_args(raw_args: Vec<OsString>, data: Option<ConfigData>) -> eyre::Result<DerivedArgs> {
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
    let arg_source = args::DerivedArgs::from_arg_matches(&matches)?;
    Ok(arg_source)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::time::Duration;

    use crate::Network;
    use base64::{engine::general_purpose::STANDARD as base64, Engine};
    use fluence_keypair::KeyPair;
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

            [root_weights]
            12D3KooWB9P1xmV3c7ZPpBemovbwCiRRTKd3Kq2jsVPQN4ZukDfy = 1

        "#).expect("Could not write in file");

        let path = file.path().display().to_string();
        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            let config = config.resolve().unwrap();
            let resolved_secret = encode_secret(&config);
            assert!(!config.node_config.allow_local_addresses);
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
            std::fs::write(&root_key_path, base64.encode(root_kp.secret().unwrap())).unwrap();
            std::fs::write(
                &builtins_key_path,
                base64.encode(builtins_kp.secret().unwrap()),
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
            endpoint = "grpc://10.10.10.10:122"
            sample_ratio = 0.1
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: Url::parse("grpc://10.10.10.10:122").unwrap(),
                    sample_ratio: Some(0.1)
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
                ("FLUENCE_TRACING__ENDPOINT", Some("grpc://10.10.10.10:122")),
            ],
            || {
                let config = load_config_with_args(vec![], None).expect("Could not load config");
                assert_eq!(
                    config.tracing,
                    Some(TracingConfig::Otlp {
                        endpoint: Url::parse("grpc://10.10.10.10:122").unwrap(),
                        sample_ratio: None
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
                OsString::from("grpc://10.10.10.10:122"),
            ];
            let config = load_config_with_args(args, None).expect("Could not load config");
            assert_eq!(
                config.tracing,
                Some(TracingConfig::Otlp {
                    endpoint: Url::parse("grpc://10.10.10.10:122").unwrap(),
                    sample_ratio: None
                })
            );
        });
    }

    #[test]
    fn load_http_port_with_env() {
        temp_env::with_vars([("FLUENCE_HTTP_PORT", Some("1234"))], || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.node_config.http_config.map(|x| x.http_port),
                Some(1234)
            );
        });
    }

    #[test]
    fn load_http_port_with_args() {
        temp_env::with_vars([("FLUENCE_HTTP_PORT", Some("1234"))], || {
            let args = vec![
                OsString::from("nox"),
                OsString::from("--http-port"),
                OsString::from("1001"),
            ];
            let config = load_config_with_args(args, None).expect("Could not load config");
            assert_eq!(
                config.node_config.http_config.map(|x| x.http_port),
                Some(1001)
            );
        });
    }

    #[test]
    fn load_env_upgrade_timeout() {
        temp_env::with_vars(
            [("FLUENCE_PROTOCOL_CONFIG__UPGRADE_TIMEOUT", Some("60s"))],
            || {
                let args = vec![];
                let config = load_config_with_args(args, None).expect("Could not load config");
                assert_eq!(
                    config.node_config.protocol_config.upgrade_timeout,
                    Duration::from_secs(60)
                );
            },
        );
    }

    #[test]
    fn load_file_upgrade_timeout() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            [protocol_config]
            upgrade_timeout = "60s"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");
            assert_eq!(
                config.node_config.protocol_config.upgrade_timeout,
                Duration::from_secs(60)
            );
        });
    }

    #[test]
    fn load_multiple_configs() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file,
            r#"
            [protocol_config]
            upgrade_timeout = "60s"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        let mut file2 = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file2,
            r#"
            allowed_binaries = [
                 "/usr/bin/curl",
                 "/usr/bin/ipfs",
                 "/usr/bin/glaze",
                 "/usr/bin/bitcoin-cli"
            ]
            websocket_port = 1234
            "#
        )
        .expect("Could not write in file");

        let path2 = file2.path().display().to_string();

        let mut file3 = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file3,
            r#"
            websocket_port = 666
            "#
        )
        .expect("Could not write in file");

        let path3 = file3.path().display().to_string();

        let mut file4 = NamedTempFile::new().expect("Could not create temp file");
        write!(
            file4,
            r#"
           aquavm_pool_size = 160
            "#
        )
        .expect("Could not write in file");

        let path4 = file4.path().display().to_string();

        let args = vec![
            OsString::from("nox"),
            OsString::from("--config"),
            OsString::from(path3.to_string()),
            OsString::from("--config"),
            OsString::from(path4.to_string()),
        ];

        temp_env::with_var(
            "FLUENCE_CONFIG",
            Some(format!("{},{}", path, path2)),
            || {
                let config = load_config_with_args(args, None).expect("Could not load config");
                assert_eq!(
                    config.node_config.protocol_config.upgrade_timeout,
                    Duration::from_secs(60)
                );
                assert_eq!(
                    config.node_config.allowed_binaries,
                    vec![
                        "/usr/bin/curl",
                        "/usr/bin/ipfs",
                        "/usr/bin/glaze",
                        "/usr/bin/bitcoin-cli"
                    ]
                );
                assert_eq!(config.node_config.listen_config.websocket_port, 666);
                assert_eq!(config.node_config.aquavm_pool_size, 160);
            },
        );
    }

    #[test]
    fn load_file_with_custom_network() {
        let mut file = NamedTempFile::new().expect("Could not create temp file");

        write!(
            file,
            r#"
            [network]
            Custom = "12458ae2e882cc71eaf2de71101c76a77a54ee89cae8897b231a8a1cb4e90f80"
            "#
        )
        .expect("Could not write in file");

        let path = file.path().display().to_string();

        temp_env::with_var("FLUENCE_CONFIG", Some(path), || {
            let config = load_config_with_args(vec![], None).expect("Could not load config");

            let expected =
                hex::decode("12458ae2e882cc71eaf2de71101c76a77a54ee89cae8897b231a8a1cb4e90f80")
                    .unwrap();
            let expected: [u8; 32] = expected.try_into().unwrap();

            assert_eq!(config.node_config.network, Network::Custom(expected));
        });
    }
}
