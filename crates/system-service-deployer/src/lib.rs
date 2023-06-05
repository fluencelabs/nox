#![feature(try_blocks)]
use eyre::eyre;
use eyre::WrapErr;
use fluence_app_service::TomlMarineConfig;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use libp2p::PeerId;
use particle_args::JError;
use particle_execution::FunctionOutcome;
use particle_modules::{AddBlueprint, ModuleRepository};
use particle_services::{ParticleAppServices, ServiceError, ServiceType};
use serde_json::{json, Value as JValue};
use server_config::{DeciderConfig, SystemServicesConfig};
use sorcerer::{get_spell_info, spell_install_inner};
use spell_event_bus::api::SpellEventBusApi;
use spell_storage::SpellStorage;
use std::collections::HashMap;
use std::time::Duration;

const SYSTEM_SERVICE_DEPLOYER_TTL: u64 = 60_000;

// TODO: do smth with naming
enum DeployedService {
    Created(String),
    Existing(String),
}

// TODO: do smth with naming
enum FindSpell {
    NeedUpdate(String),
    NoUpdate(String),
    NotFound,
}

// This is supposed to be in a separate lib for all system services crates
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
        self.deploy_aqua_ipfs()
            .map_err(|e| JError::new(e.to_string()))?;
        self.deploy_trust_graph()
            .map_err(|e| JError::new(e.to_string()))?;

        self.deploy_registry().await?;

        self.deploy_connector()
            .map_err(|e| JError::new(e.to_string()))?;
        self.deploy_decider().await
    }

    fn deploy_aqua_ipfs(&self) -> eyre::Result<()> {
        let aqua_ipfs_distro = Self::get_ipfs_service_distro();
        let service_id = match self.deploy_system_service(aqua_ipfs_distro)? {
            DeployedService::Existing(id) => {
                log::info!("found `aqua-ipfs` [{id}] service, no need to redeploy");
                return Ok(());
            }
            DeployedService::Created(id) => {
                log::info!("deployed `aqua-ipfs` [{id}] service");
                id
            }
        };

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
        let deployed = self.deploy_system_service(connector_distro)?;
        match deployed {
            DeployedService::Existing(id) => {
                log::info!("found `fluence_aurora_connector` [{id}] service, no need to redeploy");
            }
            DeployedService::Created(id) => {
                log::info!("deployed `fluence_aurora_connector` [{id}] service");
            }
        };
        Ok(())
    }

    async fn deploy_decider(&self) -> Result<(), JError> {
        let decider_distro = Self::get_decider_distro(self.config.decider.clone());
        let deployed = self.deploy_system_spell(decider_distro).await?;
        match deployed {
            DeployedService::Existing(id) => {
                log::info!("found `decider` [{id}] spell, no need to redeploy");
            }
            DeployedService::Created(id) => {
                log::info!("deployed `decider` [{id}] spell");
            }
        };

        Ok(())
    }

    async fn deploy_registry(&self) -> Result<(), JError> {
        let (registry_distro, registry_spell_distros) = Self::get_registry_distro();
        let deployed = self
            .deploy_system_service(registry_distro)
            .map_err(|e| JError::new(e.to_string()))?;
        let registry_service_id = match deployed {
            DeployedService::Existing(id) => {
                log::info!("found `registry` [{id}] service, no need to redeploy",);
                return Ok(());
            }
            DeployedService::Created(id) => id,
        };

        log::info!("deployed `registry` [{}] service", registry_service_id);
        for spell_distro in registry_spell_distros {
            let spell_name = spell_distro.name.to_string();
            let deployed = self.deploy_system_spell(spell_distro).await?;
            let spell_id = match deployed {
                DeployedService::Existing(id) => {
                    log::info!("found `{spell_name}` [{id}] spell, no need to redeploy");
                    continue;
                }
                DeployedService::Created(id) => id,
            };

            log::info!("deployed `{spell_name}` [{spell_id}] spell");
        }
        Ok(())
    }

    fn deploy_trust_graph(&self) -> eyre::Result<()> {
        let service_distro = Self::get_trust_graph_distro();
        let service_id = match self.deploy_system_service(service_distro)? {
            DeployedService::Existing(id) => {
                log::info!("found `trust-graph` [{id}] service, no need to redeploy",);
                return Ok(());
            }
            DeployedService::Created(id) => id,
        };
        log::info!("deployed `trust-graph` [{service_id}] service");

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
        log::info!("initialized `trust-graph` [{service_id}] service");
        Ok(())
    }

    // The plan is to move this to the corresponding crates
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
        let decider_config = decider_distro_test::DeciderConfig {
            worker_period_sec: decider_settings.worker_period_sec,
            worker_ipfs_multiaddr: decider_settings.worker_ipfs_multiaddr,
            //decider_period_sec: decider_settings.decider_period_sec,
            chain_network: decider_settings.network_api_endpoint,
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
            self.root_worker_id,
            Duration::from_millis(SYSTEM_SERVICE_DEPLOYER_TTL),
        );
        // similar to process_func_outcome in sorcerer/src/utils.rs, but that func is
        // to specialized to spell specific
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

    async fn deploy_system_spell(
        &self,
        spell_distro: SpellDistro,
    ) -> Result<DeployedService, JError> {
        match self.find_same_spell(&spell_distro) {
            FindSpell::NeedUpdate(spell_id) => {
                let empty_config = TriggerConfig::default();
                // Stop old spell
                let result =
                    self.call_service(&spell_id, "set_trigger_config", vec![json!(empty_config)]);
                if let Err(err) = result {
                    log::warn!("couldn't stop the old spell {}: {err}", spell_distro.name);
                }
            }
            FindSpell::NoUpdate(spell_id) => {
                return Ok(DeployedService::Existing(spell_id));
            }
            FindSpell::NotFound => {}
        }

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

        Ok(DeployedService::Created(spell_id))
    }

    // Two spells are the same if
    // - they have the same alias
    //
    // Need to redeploy (stop the old one, create a new one) a spell if
    // - the script is different
    // - the trigger config is different
    fn find_same_spell(&self, new_spell: &SpellDistro) -> FindSpell {
        let existing_spell =
            self.services
                .get_service_info("", self.root_worker_id, new_spell.name.to_string());
        let spell = match existing_spell {
            Ok(spell) => spell,
            Err(ServiceError::NoSuchService(_)) => {
                log::debug!("no existing spell found for {}", new_spell.name);
                return FindSpell::NotFound;
            }
            Err(err) => {
                log::error!(
                    "can't obtain details on a spell {} (will create a new one): {err}",
                    new_spell.name
                );
                return FindSpell::NotFound;
            }
        };
        if spell.service_type != ServiceType::Spell {
            log::warn!(
                "alias `{}` already used for a service, but now intended for a spell",
                new_spell.name,
            );
            return FindSpell::NotFound;
        }

        // Request a script and a trigger config from the spell
        let spell_info = get_spell_info(
            &self.services,
            self.root_worker_id,
            self.root_worker_id,
            SYSTEM_SERVICE_DEPLOYER_TTL,
            spell.id.clone(),
        );
        let spell_info = match spell_info {
            Err(err) => {
                log::error!(
                    "can't obtain details on existing spell {} (will try to update): {err}",
                    new_spell.name
                );
                return FindSpell::NeedUpdate(spell.id);
            }
            Ok(s) => s,
        };

        if spell_info.script != new_spell.air {
            log::debug!(
                "found old {} spell but with a different script; updating the spell",
                new_spell.name
            );
            return FindSpell::NeedUpdate(spell.id);
        }
        if spell_info.trigger_config != new_spell.trigger_config {
            log::debug!(
                "found old {} spell but with a different trigger config; updating the spell",
                new_spell.name
            );
            return FindSpell::NeedUpdate(spell.id);
        }

        FindSpell::NoUpdate(spell.id)
    }

    fn deploy_system_service(
        &self,
        service_distro: ServiceDistro,
    ) -> eyre::Result<DeployedService> {
        let service_name = service_distro.name;
        let blueprint_id = self.add_modules(service_distro)?;

        if let Some(service_id) = self.find_same_service(service_name.to_string(), &blueprint_id) {
            return Ok(DeployedService::Existing(service_id));
        }

        let service_id = self.services.create_service(
            ServiceType::Service,
            blueprint_id,
            self.root_worker_id,
            self.root_worker_id,
        )?;
        self.services.add_alias(
            service_name.to_string(),
            self.root_worker_id,
            service_id.clone(),
            self.management_id,
        )?;
        Ok(DeployedService::Created(service_id))
    }

    fn find_same_service(&self, service_name: String, blueprint_id: &str) -> Option<String> {
        // Check that the service exist and has the same blueprint.
        // In this case, we don't create a new one.
        let existing_service =
            self.services
                .get_service_info("", self.root_worker_id, service_name.to_string());
        if let Ok(service) = existing_service {
            if service.blueprint_id == blueprint_id {
                return Some(service.id);
            }
        }
        None
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
