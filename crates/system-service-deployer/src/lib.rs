#![feature(try_blocks)]
use eyre::eyre;
use eyre::WrapErr;
use fluence_app_service::TomlMarineConfig;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use libp2p::PeerId;
use particle_args::JError;
use particle_execution::FunctionOutcome;
use particle_modules::{AddBlueprint, ModuleRepository};
use particle_services::{ParticleAppServices, ServiceType};
use serde_json::{json, Value as JValue};
use server_config::{DeciderConfig, SystemServicesConfig};
use spell_event_bus::api::SpellEventBusApi;
use spell_storage::SpellStorage;
use std::collections::HashMap;
use std::time::Duration;

const SYSTEM_SERVICE_DEPLOYER_TTL: u64 = 60_000;

struct ServiceDistro {
    modules: HashMap<&'static str, &'static [u8]>,
    config: &'static [u8],
    name: &'static str,
}

struct SpellDistro {
    name: &'static str,
    air: &'static str,
    kv: HashMap<&'static str, JValue>,
    trigger_config: TriggerConfig,
}

pub struct SystemServiceDeployer {
    // These fields are used for deploying system services
    services: ParticleAppServices,
    modules_repo: ModuleRepository,
    // These fields are used for deploying system spells
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
    // These fields are used for deploying services and spells from the node name
    root_worker_id: PeerId,
    management_id: PeerId,

    config: SystemServicesConfig,
}

impl SystemServiceDeployer {
    pub fn new(
        services: ParticleAppServices,
        modules_repo: ModuleRepository,
        spell_storage: SpellStorage,
        spell_event_bus_api: SpellEventBusApi,
        root_worker_id: PeerId,
        management_id: PeerId,
        config: SystemServicesConfig,
    ) -> Self {
        Self {
            services,
            modules_repo,
            spell_storage,
            spell_event_bus_api,
            root_worker_id,
            management_id,
            config,
        }
    }

    pub async fn deploy_system_services(&self) -> Result<(), JError> {
        try {
            self.deploy_aqua_ipfs()
                .map_err(|e| JError::new(e.to_string()))?;
            self.deploy_trust_graph()
                .map_err(|e| JError::new(e.to_string()))?;

            self.deploy_registry().await?;

            self.deploy_connector()
                .map_err(|e| JError::new(e.to_string()))?;
            self.deploy_decider().await?
        }
    }

    fn deploy_aqua_ipfs(&self) -> eyre::Result<()> {
        let aqua_ipfs_distro = Self::get_ipfs_service_distro();
        let service_id = self.deploy_system_service(aqua_ipfs_distro)?;
        log::info!("deployed `aqua-ipfs` [{}] service", service_id);

        self.call_service(
            "aqua-ipfs",
            "set_local_api_multiaddr",
            vec![json!(self.config.aqua_ipfs.local_api_multiaddr)],
        )
        .map_err(|e| eyre!(e.to_string()))?;

        self.call_service(
            "aqua-ipfs",
            "set_external_api_multiaddr",
            vec![json!(self.config.aqua_ipfs.external_api_multiaddr)],
        )
        .map_err(|e| eyre!(e.to_string()))?;
        log::info!("initialized `aqua-ipfs` [{}] service", service_id);

        Ok(())
    }

    fn deploy_connector(&self) -> eyre::Result<()> {
        let connector_distro = Self::get_connector_distro();
        let service_id = self.deploy_system_service(connector_distro)?;
        log::info!(
            "deployed `fluence_aurora_connector` [{}] service",
            service_id
        );
        Ok(())
    }

    async fn deploy_decider(&self) -> Result<(), JError> {
        let decider_distro = Self::get_decider_distro(self.config.decider.clone());
        let spell_id = self.deploy_system_spell(decider_distro).await?;
        log::info!("deployed `decider` [{}] spell", spell_id);
        Ok(())
    }

    async fn deploy_registry(&self) -> Result<(), JError> {
        let (registry_distro, registry_spell_distros) = Self::get_registry_distro();
        let registry_service_id = self
            .deploy_system_service(registry_distro)
            .map_err(|e| JError::new(e.to_string()))?;
        log::info!("deployed `registry` [{}] service", registry_service_id);
        for spell_distro in registry_spell_distros {
            let spell_name = spell_distro.name.to_string();
            let spell_id = self.deploy_system_spell(spell_distro).await?;
            log::info!("deployed `{}` [{}] spell", spell_name, spell_id);
        }
        Ok(())
    }

    fn deploy_trust_graph(&self) -> eyre::Result<()> {
        let service_distro = Self::get_trust_graph_distro();
        let service_id = self.deploy_system_service(service_distro)?;
        log::info!("deployed `trust-graph` [{}] service", service_id);

        let certs = &trust_graph_distro_test::KRAS_CERTS;

        self.call_service(
            "trust-graph",
            "set_root",
            vec![json!(certs.root_node), json!(certs.max_chain_length)],
        )
        .map_err(|e| eyre!(e.to_string()))?;

        let timestamp = now_millis::now_sec();
        for cert_chain in &certs.certs {
            self.call_service(
                "trust-graph",
                "insert_cert",
                vec![json!(cert_chain), json!(timestamp)],
            )
            .map_err(|e| eyre!(e.to_string()))?;
        }
        log::info!("initialized `trust-graph` [{}] service", service_id);
        Ok(())
    }

    fn get_trust_graph_distro() -> ServiceDistro {
        use trust_graph_distro_test::*;
        ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: "trust-graph",
        }
    }

    fn get_registry_distro() -> (ServiceDistro, Vec<SpellDistro>) {
        use registry_distro_test::*;

        let distro = ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: "registry",
        };
        let spells_distro = scripts()
            .into_iter()
            .map(|script| {
                let mut trigger_config = TriggerConfig::default();
                trigger_config.clock.start_sec = 1;
                trigger_config.clock.period_sec = script.period_sec;
                SpellDistro {
                    name: script.name,
                    air: script.air,
                    kv: HashMap::new(),
                    trigger_config,
                }
            })
            .collect::<_>();
        (distro, spells_distro)
    }

    fn get_ipfs_service_distro() -> ServiceDistro {
        use aqua_ipfs_distro_test::*;
        ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: "aqua-ipfs",
        }
    }

    fn get_connector_distro() -> ServiceDistro {
        use decider_distro_test::*;
        let connector_service_distro = connector_service_modules();
        ServiceDistro {
            modules: connector_service_distro.modules,
            config: connector_service_distro.config,
            name: connector_service_distro.name,
        }
    }

    fn get_decider_distro(decider_settings: DeciderConfig) -> SpellDistro {
        use decider_distro_test::*;

        let decider_config = decider_distro_test::DeciderConfig {
            worker_period_sec: decider_settings.worker_period_sec,
            worker_ipfs_multiaddr: decider_settings.worker_ipfs_multiaddr,
            decider_period_sec: decider_settings.decider_period_sec,
            chain_network: decider_settings.network_name,
            chain_contract_addr: decider_settings.contract_address_hex,
            chain_contract_block_hex: decider_settings.contract_block_hex,
        };
        let decider_spell_distro = decider_distro_test::decider_spell(decider_config);
        let mut decider_trigger_config = TriggerConfig::default();
        decider_trigger_config.clock.start_sec = 1;
        decider_trigger_config.clock.period_sec = decider_settings.decider_period_sec;
        SpellDistro {
            name: "decider",
            air: decider_spell_distro.air,
            kv: decider_spell_distro.kv,
            trigger_config: decider_trigger_config,
        }
    }

    fn call_service(
        &self,
        service_id: &str,
        function_name: &'static str,
        args: Vec<JValue>,
    ) -> Result<(), JError> {
        let result = self.services.call_function(
            self.root_worker_id,
            &service_id,
            function_name,
            args,
            None,
            self.management_id,
            Duration::from_millis(SYSTEM_SERVICE_DEPLOYER_TTL),
        );
        match result {
            FunctionOutcome::Ok(result) => {
                let call_result: Option<Result<_, _>> = try {
                    let result = result.as_object()?;
                    let is_success = result["success"].as_bool()?;
                    if !is_success {
                        if let Some(error) = result["error"].as_str() {
                            Err(JError::new(format!(
                                "Call {service_id}.{function_name} returned error: {}",
                                error
                            )))
                        } else {
                            Err(JError::new(format!(
                                "Call {service_id}.{function_name} returned error"
                            )))
                        }
                    } else {
                        Ok(())
                    }
                };
                call_result.unwrap_or_else(|| {
                    Err(JError::new(format!(
                        "Call {service_id}.{function_name} return invalid result: {result}"
                    )))
                })
            }
            FunctionOutcome::NotDefined { .. } => Err(JError::new(format!(
                "Service {service_id} ({function_name}) not found"
            ))),
            FunctionOutcome::Empty => Err(JError::new(format!(
                "Call {service_id}.{function_name} didn't return any result"
            ))),
            FunctionOutcome::Err(err) => Err(err),
        }
    }

    async fn deploy_system_spell(&self, spell_distro: SpellDistro) -> Result<String, JError> {
        use sorcerer::spell_install_inner;

        let spell_id = spell_install_inner(
            &self.services,
            &self.spell_storage,
            &self.spell_event_bus_api,
            self.root_worker_id,
            self.root_worker_id,
            self.root_worker_id,
            "dummy-particle-id".to_string(),
            SYSTEM_SERVICE_DEPLOYER_TTL,
            spell_distro.trigger_config,
            spell_distro.air.to_string(),
            json!(spell_distro.kv),
        )
        .await?;
        self.services.add_alias(
            spell_distro.name.to_string(),
            self.root_worker_id,
            spell_id.clone(),
            self.management_id,
        )?;

        Ok(spell_id)
    }

    fn deploy_system_service(&self, service_distro: ServiceDistro) -> eyre::Result<String> {
        let service_name = service_distro.name;
        let blueprint_id = self.add_modules(service_distro)?;

        let service_id = self.services.create_service(
            ServiceType::Service,
            blueprint_id,
            self.management_id,
            self.root_worker_id,
        )?;
        self.services.add_alias(
            service_name.to_string(),
            self.root_worker_id,
            service_id.clone(),
            self.management_id,
        )?;
        Ok(service_id)
    }

    fn add_modules(&self, service_distro: ServiceDistro) -> eyre::Result<String> {
        let marine_config: TomlMarineConfig = toml::from_slice(service_distro.config)?;
        let mut hashes = Vec::new();
        for config in marine_config.module {
            let name = config.name.clone();
            // TODO: introduce nice errors for this
            let module = service_distro
                .modules
                .get(name.as_str())
                .ok_or(eyre!(format!(
                    "there's no module {name} in the given modules map for {}",
                    service_distro.name
                )))?;
            let hash = self
                .modules_repo
                .add_module(module.to_vec(), config)
                .context(format!(
                    "adding module {name} of service `{}`",
                    service_distro.name
                ))?;
            hashes.push(hash)
        }
        let blueprint_id = self
            .modules_repo
            .add_blueprint(AddBlueprint::new(service_distro.name.to_string(), hashes))?;
        Ok(blueprint_id)
    }
}
