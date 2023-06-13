#![feature(try_blocks)]
use eyre::eyre;
use fluence_app_service::TomlMarineConfig;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use libp2p::PeerId;
use particle_args::JError;
use particle_execution::FunctionOutcome;
use particle_modules::{AddBlueprint, ModuleRepository};
use particle_services::{ParticleAppServices, ServiceError, ServiceType};
use serde_json::{json, Value as JValue};
use server_config::system_services_config::ServiceKey::*;
use server_config::{system_services_config::ServiceKey, DeciderConfig, SystemServicesConfig};
use sorcerer::{get_spell_info, install_spell, remove_spell};
use spell_event_bus::api::SpellEventBusApi;
use spell_storage::SpellStorage;
use std::collections::HashMap;
use std::time::Duration;

const DEPLOYER_TTL: u64 = 60_000;

const DEPLOYER_PARTICLE_ID: &str = "system-services-deployment";

// A status of a service/spell after deployment
enum ServiceStatus {
    // Id of a newly created service
    Created(String),
    // Id of a already existing service
    Existing(String),
}

// Status of the service or spell before deployment
enum ServiceUpdateStatus {
    // A service is found and we need to update it
    NeedUpdate(String),
    // A service is found and it's up to date
    NoUpdate(String),
    // A service isn't found
    NotFound,
}

// This is supposed to be in a separate lib for all system services crates
struct ServiceDistro {
    modules: HashMap<&'static str, &'static [u8]>,
    config: &'static [u8],
    name: String,
}

struct SpellDistro {
    name: String,
    air: &'static str,
    kv: HashMap<&'static str, JValue>,
    trigger_config: TriggerConfig,
}

pub struct Deployer {
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

impl Deployer {
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

    async fn deploy_system_service(&self, key: &ServiceKey) -> Result<(), JError> {
        match key {
            AquaIpfs => self.deploy_aqua_ipfs(),
            TrustGraph => self.deploy_trust_graph(),
            Registry => self.deploy_registry().await,
            Decider => {
                self.deploy_connector()?;
                self.deploy_decider().await
            }
            x => {
                log::error!("installation of a service `{x}` is not implemented",);
                Ok(())
            }
        }
    }

    pub async fn deploy_system_services(&self) -> Result<(), JError> {
        for service in &self.config.enable {
            self.deploy_system_service(&service).await?;
        }
        Ok(())
    }

    fn deploy_aqua_ipfs(&self) -> Result<(), JError> {
        let aqua_ipfs_distro = Self::get_ipfs_service_distro();
        let service_name = aqua_ipfs_distro.name.clone();
        let service_id = match self.deploy_service_common(aqua_ipfs_distro)? {
            ServiceStatus::Existing(_id) => {
                return Ok(());
            }
            ServiceStatus::Created(id) => id,
        };

        let set_local_result = self.call_service(
            &service_name,
            "set_local_api_multiaddr",
            vec![json!(self.config.aqua_ipfs.local_api_multiaddr)],
        );

        let set_external_result = self.call_service(
            &service_name,
            "set_external_api_multiaddr",
            vec![json!(self.config.aqua_ipfs.external_api_multiaddr)],
        );

        // try to set local and external api multiaddrs, and only then produce an error
        set_local_result?;
        set_external_result?;

        log::info!("initialized `aqua-ipfs` [{}] service", service_id);

        Ok(())
    }

    fn deploy_connector(&self) -> Result<(), JError> {
        let connector_distro = Self::get_connector_distro();
        let _deployed = self.deploy_service_common(connector_distro)?;
        Ok(())
    }

    async fn deploy_decider(&self) -> Result<(), JError> {
        let decider_distro = Self::get_decider_distro(self.config.decider.clone());
        let _deployed = self.deploy_system_spell(decider_distro).await?;

        Ok(())
    }

    async fn deploy_registry(&self) -> Result<(), JError> {
        let (registry_distro, registry_spell_distros) = Self::get_registry_distro();
        let _deployed = self
            .deploy_service_common(registry_distro)
            .map_err(|e| JError::new(e.to_string()))?;

        for spell_distro in registry_spell_distros {
            let _deployed = self.deploy_system_spell(spell_distro).await?;
        }
        Ok(())
    }

    fn deploy_trust_graph(&self) -> Result<(), JError> {
        let service_distro = Self::get_trust_graph_distro();
        let service_name = service_distro.name.clone();
        let service_id = match self.deploy_service_common(service_distro)? {
            ServiceStatus::Existing(_id) => {
                return Ok(());
            }
            ServiceStatus::Created(id) => id,
        };

        let certs = &trust_graph_distro::KRAS_CERTS;

        self.call_service(
            &service_name,
            "set_root",
            vec![json!(certs.root_node), json!(certs.max_chain_length)],
        )?;

        let timestamp = now_millis::now_sec();
        for cert_chain in &certs.certs {
            self.call_service(
                &service_name,
                "insert_cert",
                vec![json!(cert_chain), json!(timestamp)],
            )?;
        }
        log::info!("initialized `{service_name}` [{service_id}] service");
        Ok(())
    }

    // The plan is to move this to the corresponding crates
    fn get_trust_graph_distro() -> ServiceDistro {
        use trust_graph_distro::*;
        ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: TrustGraph.to_string(),
        }
    }

    fn get_registry_distro() -> (ServiceDistro, Vec<SpellDistro>) {
        use registry_distro::*;

        let distro = ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: Registry.to_string(),
        };
        let spells_distro = scripts()
            .into_iter()
            .map(|script| {
                let mut trigger_config = TriggerConfig::default();
                trigger_config.clock.start_sec = 1;
                trigger_config.clock.period_sec = script.period_sec;
                SpellDistro {
                    name: script.name.to_string(),
                    air: script.air,
                    kv: HashMap::new(),
                    trigger_config,
                }
            })
            .collect::<_>();
        (distro, spells_distro)
    }

    fn get_ipfs_service_distro() -> ServiceDistro {
        use aqua_ipfs_distro::*;
        ServiceDistro {
            modules: modules(),
            config: CONFIG,
            name: AquaIpfs.to_string(),
        }
    }

    fn get_connector_distro() -> ServiceDistro {
        let connector_service_distro = decider_distro::connector_service_modules();
        ServiceDistro {
            modules: connector_service_distro.modules,
            config: connector_service_distro.config,
            name: connector_service_distro.name.to_string(),
        }
    }

    fn get_decider_distro(decider_settings: DeciderConfig) -> SpellDistro {
        let decider_config = decider_distro::DeciderConfig {
            worker_period_sec: decider_settings.worker_period_sec,
            worker_ipfs_multiaddr: decider_settings.worker_ipfs_multiaddr,
            chain_network: decider_settings.network_api_endpoint,
            chain_contract_addr: decider_settings.contract_address_hex,
            chain_contract_block_hex: decider_settings.contract_block_hex,
        };
        let decider_spell_distro = decider_distro::decider_spell(decider_config);
        let mut decider_trigger_config = TriggerConfig::default();
        decider_trigger_config.clock.start_sec = 1;
        decider_trigger_config.clock.period_sec = decider_settings.decider_period_sec;
        SpellDistro {
            name: Decider.to_string(),
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
            Duration::from_millis(DEPLOYER_TTL),
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
    ) -> Result<ServiceStatus, JError> {
        let spell_name = spell_distro.name.clone();
        match self.find_same_spell(&spell_distro) {
            ServiceUpdateStatus::NeedUpdate(spell_id) => {
                log::info!("found existing spell `{spell_name}` [{spell_id}]; will redeploy",);
                self.clean_old_spell(&spell_name, spell_id).await;
            }
            ServiceUpdateStatus::NoUpdate(spell_id) => {
                log::info!("found existing spell `{spell_name}` [{spell_id}]; no need to redeploy",);
                return Ok(ServiceStatus::Existing(spell_id));
            }
            ServiceUpdateStatus::NotFound => {}
        }
        let spell_id = self.deploy_spell_common(spell_distro).await?;
        log::info!("deployed a system spell `{spell_name}` [{spell_id}]",);
        Ok(ServiceStatus::Created(spell_id))
    }

    async fn clean_old_spell(&self, spell_name: &str, spell_id: String) {
        let result = remove_spell(
            DEPLOYER_PARTICLE_ID,
            &self.spell_storage,
            &self.services,
            &self.spell_event_bus_api,
            spell_id.clone(),
            self.root_worker_id,
        )
        .await;
        if let Err(err) = result {
            log::error!(
                "Failed to remove old spell `{spell_name}` [{spell_id}] (trying to stop it): {err}",
            );

            let empty_config = TriggerConfig::default();
            // Stop old spell
            let result: Result<_, JError> = try {
                // Stop the spell to avoid re-subscription
                self.call_service(&spell_id, "set_trigger_config", vec![json!(empty_config)])?;

                // Unsubscribe spell from execution
                self.spell_event_bus_api.unsubscribe(spell_id.clone()).await
            };
            if let Err(err) = result {
                log::error!(
                    "couldn't stop the old spell {spell_name} (will install new spell nevertheless): {err}",
                );
            }
        }
    }

    async fn deploy_spell_common(&self, spell_distro: SpellDistro) -> Result<String, JError> {
        let spell_id = install_spell(
            &self.services,
            &self.spell_storage,
            &self.spell_event_bus_api,
            self.root_worker_id,
            DEPLOYER_PARTICLE_ID.to_string(),
            DEPLOYER_TTL,
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

    // Two spells are the same if
    // - they have the same alias
    //
    // Need to redeploy (stop the old one, create a new one) a spell if
    // - the script is different
    // - the trigger config is different
    fn find_same_spell(&self, new_spell: &SpellDistro) -> ServiceUpdateStatus {
        let existing_spell =
            self.services
                .get_service_info("", self.root_worker_id, new_spell.name.to_string());
        let spell = match existing_spell {
            Ok(spell) => spell,
            Err(ServiceError::NoSuchService(_)) => {
                log::debug!("no existing spell found for {}", new_spell.name);
                return ServiceUpdateStatus::NotFound;
            }
            Err(err) => {
                log::error!(
                    "can't obtain details on a spell `{}` (will create a new one): {err}",
                    new_spell.name
                );
                return ServiceUpdateStatus::NotFound;
            }
        };
        if spell.service_type != ServiceType::Spell {
            log::warn!(
                "alias `{}` already used for a service [{}]; it will be used for a spell, the service won't be removed",
                new_spell.name,
                spell.id
            );
            return ServiceUpdateStatus::NotFound;
        }

        // Request a script and a trigger config from the spell
        let spell_info = get_spell_info(
            &self.services,
            self.root_worker_id,
            DEPLOYER_TTL,
            spell.id.clone(),
        );
        let spell_info = match spell_info {
            Err(err) => {
                log::error!(
                    "can't obtain details on existing spell {} (will try to update nevertheless): {err}",
                    new_spell.name
                );
                return ServiceUpdateStatus::NeedUpdate(spell.id);
            }
            Ok(s) => s,
        };

        if spell_info.script != new_spell.air {
            log::debug!(
                "found old {} spell but with a different script; updating the spell",
                new_spell.name
            );
            return ServiceUpdateStatus::NeedUpdate(spell.id);
        }
        if spell_info.trigger_config != new_spell.trigger_config {
            log::debug!(
                "found old {} spell but with a different trigger config; updating the spell",
                new_spell.name
            );
            return ServiceUpdateStatus::NeedUpdate(spell.id);
        }

        ServiceUpdateStatus::NoUpdate(spell.id)
    }

    fn deploy_service_common(
        &self,
        service_distro: ServiceDistro,
    ) -> Result<ServiceStatus, JError> {
        let service_name = service_distro.name.clone();
        let blueprint_id = self
            .add_modules(service_distro)
            .map_err(|e| JError::new(e.to_string()))?;

        match self.find_same_service(service_name.to_string(), &blueprint_id) {
            ServiceUpdateStatus::NeedUpdate(service_id) => {
                log::info!("found existing service {service_name} [{service_id}]; will redeploy");
                let result = self.services.remove_service(
                    DEPLOYER_PARTICLE_ID,
                    self.root_worker_id,
                    service_id,
                    self.root_worker_id,
                    false,
                );
                if let Err(err) = result {
                    log::error!(
                        "couldn't remove the old service {service_name} (will install new service nevertheless): {err}",
                    );
                }
            }
            ServiceUpdateStatus::NoUpdate(service_id) => {
                log::info!(
                    "found existing service {service_name} [{service_id}]; no need to redeploy"
                );
                return Ok(ServiceStatus::Existing(service_id));
            }
            ServiceUpdateStatus::NotFound => {}
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
        log::info!("deployed service {service_name} [{service_id}]");
        Ok(ServiceStatus::Created(service_id))
    }

    fn find_same_service(&self, service_name: String, blueprint_id: &str) -> ServiceUpdateStatus {
        // Check that the service exist and has the same blueprint.
        // In this case, we don't create a new one.
        let existing_service =
            self.services
                .get_service_info("", self.root_worker_id, service_name.to_string());
        if let Ok(service) = existing_service {
            if service.service_type == ServiceType::Spell {
                log::warn!(
                    "alias `{}` already used for a spell [{}]; it will be used for a new service, the spell won't be removed",
                    service_name,
                    service.id
                );
                return ServiceUpdateStatus::NotFound;
            }

            if service.blueprint_id == blueprint_id {
                ServiceUpdateStatus::NoUpdate(service.id)
            } else {
                ServiceUpdateStatus::NeedUpdate(service.id)
            }
        } else {
            ServiceUpdateStatus::NotFound
        }
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
                    "there's no module `{name}` in the given modules map for system service {}",
                    service_distro.name
                )))?;
            let hash = self
                .modules_repo
                .add_module(module.to_vec(), config)
                .map_err(|e| {
                    eyre!(
                        "error while adding module {name} of service `{}`: {:?}",
                        service_distro.name,
                        e
                    )
                })?;
            hashes.push(hash)
        }
        let blueprint_id = self
            .modules_repo
            .add_blueprint(AddBlueprint::new(service_distro.name.to_string(), hashes))?;
        Ok(blueprint_id)
    }
}
