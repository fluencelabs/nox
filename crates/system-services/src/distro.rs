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

use async_trait::async_trait;
use fluence_app_service::TomlMarineConfig;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use serde_json::json;
use server_config::system_services_config::{
    AquaIpfsConfig, DeciderConfig, RegistryConfig, ServiceKey, ServiceKey::*, SystemServicesConfig,
};
use std::collections::HashMap;
use std::sync::Arc;
use trust_graph_distro::Certs;

use crate::{
    apply_binary_path_override, CallService, Deployment, InitService, PackageDistro, ServiceDistro,
    ServiceStatus, SpellDistro,
};

#[derive(Debug, Clone)]
pub struct Versions {
    pub aqua_ipfs_version: &'static str,
    pub trust_graph_version: &'static str,
    pub registry_version: &'static str,
    pub decider_version: &'static str,
}

#[derive(Clone, Debug)]
pub struct SystemServiceDistros {
    pub(crate) distros: HashMap<String, PackageDistro>,
    pub(crate) versions: Versions,
}

impl SystemServiceDistros {
    /// With overriding existing packages
    pub fn extend(mut self, distros: Vec<PackageDistro>) -> Self {
        for distro in distros {
            self.distros.insert(distro.name.clone(), distro);
        }
        self
    }

    pub fn default_from(config: SystemServicesConfig) -> eyre::Result<Self> {
        log::warn!("{:?}", config);
        let distros: HashMap<String, PackageDistro> = config
            .enable
            .iter()
            .map(move |key| {
                let distro = match key {
                    AquaIpfs => default_aqua_ipfs_distro(&config.aqua_ipfs),
                    TrustGraph => default_trust_graph_distro(),
                    Registry => default_registry_distro(&config.registry),
                    Decider => default_decider_distro(&config.decider),
                };
                distro.map(move |d| (d.name.clone(), d))
            })
            .collect::<eyre::Result<_>>()?;

        let versions = Self::versions_from(&distros);

        Ok(SystemServiceDistros { distros, versions })
    }

    fn default_versions() -> Versions {
        Versions {
            aqua_ipfs_version: aqua_ipfs_distro::VERSION,
            trust_graph_version: trust_graph_distro::VERSION,
            registry_version: registry_distro::VERSION,
            decider_version: decider_distro::VERSION,
        }
    }

    fn versions_from(packages: &HashMap<String, PackageDistro>) -> Versions {
        let mut versions = Self::default_versions();
        for (name, package) in packages {
            match ServiceKey::from_string(name) {
                Some(AquaIpfs) => {
                    versions.aqua_ipfs_version = package.version;
                }
                Some(Registry) => {
                    versions.registry_version = package.version;
                }
                Some(TrustGraph) => {
                    versions.trust_graph_version = package.version;
                }
                Some(Decider) => {
                    versions.decider_version = package.version;
                }
                _ => {}
            }
        }

        versions
    }
}

struct TrustGraphInit<'a> {
    name: String,
    certs: &'a Certs,
}

#[async_trait]
impl<'a> InitService for TrustGraphInit<'a> {
    async fn init(
        &self,
        call_service: &dyn CallService,
        deployment: Deployment,
    ) -> eyre::Result<()> {
        if let Some(ServiceStatus::Created(id)) = deployment.services.get(&self.name) {
            call_service
                .call(
                    self.name.clone(),
                    "set_root".to_string(),
                    vec![
                        json!(self.certs.root_node),
                        json!(self.certs.max_chain_length),
                    ],
                )
                .await?;

            let timestamp = now_millis::now_sec();
            for cert_chain in &self.certs.certs {
                call_service
                    .call(
                        self.name.clone(),
                        "insert_cert".to_string(),
                        vec![json!(cert_chain), json!(timestamp)],
                    )
                    .await?;
            }
            tracing::info!(
                service_id = id,
                service_alias = self.name,
                "initialized service"
            );
        }
        Ok(()) as eyre::Result<()>
    }
}

pub fn default_trust_graph_distro<'a>() -> eyre::Result<PackageDistro> {
    use trust_graph_distro::*;

    let config: TomlMarineConfig = toml_edit::de::from_slice(CONFIG)?;
    let service_distro = ServiceDistro {
        modules: modules(),
        config,
        name: TrustGraph.to_string(),
    };
    let certs: &'static Certs = &trust_graph_distro::KRAS_CERTS;
    let init = TrustGraphInit {
        name: TrustGraph.to_string(),
        certs,
    };
    let package = PackageDistro {
        name: TrustGraph.to_string(),
        version: VERSION,
        services: vec![service_distro],
        spells: vec![],
        init: Some(Arc::new(Box::new(init))),
    };
    Ok(package)
}

struct AquaIpfsConfigInit {
    local_api_multiaddr: String,
    external_api_multiaddr: String,
    name: String,
}

#[async_trait]
impl InitService for AquaIpfsConfigInit {
    async fn init(
        &self,
        call_service: &dyn CallService,
        deployment: Deployment,
    ) -> eyre::Result<()> {
        if let Some(ServiceStatus::Created(id) | ServiceStatus::Existing(id)) =
            deployment.services.get(&self.name)
        {
            let set_local_result = call_service
                .call(
                    self.name.clone(),
                    "set_local_api_multiaddr".to_string(),
                    vec![json!(self.local_api_multiaddr)],
                )
                .await;

            let set_external_result = call_service
                .call(
                    self.name.clone(),
                    "set_external_api_multiaddr".to_string(),
                    vec![json!(self.external_api_multiaddr)],
                )
                .await;

            // try to set local and external api multiaddrs, and only then produce an error
            set_local_result?;
            set_external_result?;

            tracing::info!(
                service_id = id,
                service_alias = self.name,
                "initialized service"
            );
        }
        Ok(())
    }
}

pub fn default_aqua_ipfs_distro(config: &AquaIpfsConfig) -> eyre::Result<PackageDistro> {
    use aqua_ipfs_distro::*;

    let mut marine_config: TomlMarineConfig = toml_edit::de::from_slice(CONFIG)?;
    apply_binary_path_override(
        &mut marine_config,
        "ipfs_effector",
        "ipfs",
        config.ipfs_binary_path.clone(),
    );

    let service_distro = ServiceDistro {
        modules: modules(),
        config: marine_config,
        name: AquaIpfs.to_string(),
    };

    let local_api_multiaddr = config.local_api_multiaddr.clone();
    let external_api_multiaddr = config.external_api_multiaddr.clone();

    let init = AquaIpfsConfigInit {
        local_api_multiaddr,
        external_api_multiaddr,
        name: AquaIpfs.to_string(),
    };

    let package = PackageDistro {
        name: AquaIpfs.to_string(),
        version: VERSION,
        services: vec![service_distro],
        spells: vec![],
        init: Some(Arc::new(Box::new(init))),
    };

    Ok(package)
}

pub fn default_registry_distro(config: &RegistryConfig) -> eyre::Result<PackageDistro> {
    let marine_config: TomlMarineConfig = toml_edit::de::from_slice(registry_distro::CONFIG)?;
    let service_distro = ServiceDistro {
        modules: registry_distro::modules(),
        config: marine_config,
        name: Registry.to_string(),
    };

    let registry_config = registry_distro::RegistryConfig {
        expired_interval: config.expired_period_sec,
        renew_interval: config.renew_period_sec,
        replicate_interval: config.replicate_period_sec,
    };
    let spell_distro = registry_distro::registry_spell(registry_config);
    let mut trigger_config = TriggerConfig::default();
    trigger_config.clock.start_sec = 1;
    trigger_config.clock.period_sec = config.registry_period_sec;
    let spell_distro = SpellDistro {
        name: "registry-spell".to_string(),
        air: spell_distro.air,
        kv: spell_distro.init_data,
        trigger_config,
    };
    let package = PackageDistro {
        name: Registry.to_string(),
        version: registry_distro::VERSION,
        services: vec![service_distro],
        spells: vec![spell_distro],
        init: None,
    };
    Ok(package)
}

pub fn default_decider_distro(decider_config: &DeciderConfig) -> eyre::Result<PackageDistro> {
    // prepare decider
    let decider_settings = decider_distro::DeciderConfig {
        worker_period_sec: decider_config.worker_period_sec,
        worker_ipfs_multiaddr: decider_config.worker_ipfs_multiaddr.clone(),
    };
    let decider_spell_distro = decider_distro::decider_spell(decider_settings);
    let mut decider_trigger_config = TriggerConfig::default();
    decider_trigger_config.clock.start_sec = 1;
    decider_trigger_config.clock.period_sec = decider_config.decider_period_sec;
    let spell_distro = SpellDistro {
        name: Decider.to_string(),
        air: decider_spell_distro.air,
        kv: decider_spell_distro.kv,
        trigger_config: decider_trigger_config,
    };

    let package = PackageDistro {
        name: Decider.to_string(),
        version: decider_distro::VERSION,
        services: vec![],
        spells: vec![spell_distro],
        init: None,
    };
    Ok(package)
}
