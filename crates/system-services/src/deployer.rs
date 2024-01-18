use crate::distro::*;
use crate::CallService;
use crate::{DeploymentStatus, PackageDistro, ServiceDistro, ServiceStatus, SpellDistro};
use eyre::eyre;
use libp2p::PeerId;
use particle_execution::FunctionOutcome;
use particle_modules::{AddBlueprint, ModuleRepository};
use particle_services::{ParticleAppServices, ServiceError, ServiceType};
use serde_json::{json, Value as JValue};
use sorcerer::{install_spell, remove_spell};
use spell_event_bus::api::{SpellEventBusApi, SpellId};
use spell_service_api::{CallParams, SpellServiceApi};
use spell_storage::SpellStorage;
use std::collections::HashMap;
use std::time::Duration;
use uuid_utils::uuid;

const DEPLOYER_TTL: Duration = Duration::from_millis(60_000);

const DEPLOYER_PARTICLE_ID: &str = "system-services-deployment";

fn get_deployer_particle_id() -> String {
    format!("{}_{}", DEPLOYER_PARTICLE_ID, uuid())
}

// Status of the service or spell before deployment
#[derive(Clone, Debug)]
enum ServiceUpdateStatus {
    // A service is found and we need to update it
    NeedUpdate(String),
    // A service is found and it's up to date
    NoUpdate(String),
    // A service isn't found
    NotFound,
}

#[derive(Clone, Debug)]
pub struct Deployer {
    // These fields are used for deploying system services
    services: ParticleAppServices,
    modules_repo: ModuleRepository,
    // These fields are used for deploying system spells
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
    spells_api: SpellServiceApi,
    // These fields are used for deploying services and spells from the node name
    root_worker_id: PeerId,
    management_id: PeerId,

    system_service_distros: SystemServiceDistros,
}

impl Deployer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        services: ParticleAppServices,
        modules_repo: ModuleRepository,
        spell_storage: SpellStorage,
        spell_event_bus_api: SpellEventBusApi,
        spell_service_api: SpellServiceApi,
        root_worker_id: PeerId,
        management_id: PeerId,
        system_service_distros: SystemServiceDistros,
    ) -> Self {
        Self {
            services,
            modules_repo,
            spell_storage,
            spell_event_bus_api,
            spells_api: spell_service_api,
            root_worker_id,
            management_id,

            system_service_distros,
        }
    }
    pub fn versions(&self) -> Versions {
        self.system_service_distros.versions.clone()
    }

    pub async fn deploy_system_services(self) -> eyre::Result<()> {
        // TODO: Can we do this without cloning?

        let services = self.services.clone();
        let root_worker_id = self.root_worker_id;
        let call: CallService = Box::new(move |srv, fnc, args| {
            call_service(&services, root_worker_id, &srv, &fnc, args)
        });

        for distro in self.system_service_distros.distros.values() {
            self.deploy_package(&call, distro.clone()).await?;
        }
        Ok(())
    }

    async fn deploy_package(&self, call: &CallService, package: PackageDistro) -> eyre::Result<()> {
        let mut services = HashMap::new();
        for service_distro in package.services {
            let name = service_distro.name.clone();
            let result = self.deploy_service_common(service_distro).await?;
            services.insert(name, result);
        }

        let mut spells = HashMap::new();
        for spell_distro in package.spells {
            let name = spell_distro.name.clone();
            let result = self.deploy_system_spell(spell_distro).await?;
            spells.insert(name, result);
        }
        let status = DeploymentStatus { services, spells };

        if let Some(init) = package.init {
            init(call, status)?;
        }

        Ok(())
    }

    async fn deploy_system_spell(&self, spell_distro: SpellDistro) -> eyre::Result<ServiceStatus> {
        let spell_name = spell_distro.name.clone();
        match self.find_same_spell(&spell_distro) {
            Some(spell_id) => {
                tracing::debug!(
                    spell_name,
                    spell_id,
                    "found existing spell that needs to be updated; will try to update script, trigger config and init data",
                );
                match self.update_spell(&spell_distro, &spell_id).await {
                    Err(err) => {
                        tracing::warn!(
                            spell_id,
                            spell_name,
                            "can't update a spell (will redeploy it): {err}"
                        );
                        self.remove_old_spell(&spell_name, &spell_id).await?;

                        let spell_id = self.deploy_spell_common(spell_distro).await?;
                        tracing::info!(spell_name, spell_id, "redeployed a system spell",);
                        Ok(ServiceStatus::Created(spell_id))
                    }
                    Ok(()) => {
                        tracing::info!(spell_name, spell_id, "updated a system spell");
                        Ok(ServiceStatus::Existing(spell_id))
                    }
                }
            }

            None => {
                let spell_id = self.deploy_spell_common(spell_distro).await?;
                tracing::info!(spell_name, spell_id, "deployed a system spell",);
                Ok(ServiceStatus::Created(spell_id))
            }
        }
    }

    // Updating spell is:
    // - updating script
    // - updating trigger config
    // - updating kv
    async fn update_spell(&self, spell_distro: &SpellDistro, spell_id: &str) -> eyre::Result<()> {
        // stop spell
        let result = self
            .spell_event_bus_api
            .unsubscribe(spell_id.to_string())
            .await;
        if let Err(err) = result {
            tracing::warn!(
                spell_id,
                "failed to unsubscribe spell (will try to update the spell nevertheless): {err}"
            );
        }

        let trigger_config = spell_event_bus::api::from_user_config(&spell_distro.trigger_config)?;
        let params = CallParams::local(spell_id.to_string(), self.root_worker_id, DEPLOYER_TTL);
        // update trigger config
        let config = spell_distro.trigger_config.clone();
        self.spells_api.set_trigger_config(params.clone(), config)?;
        // update spell script
        let air = spell_distro.air.to_string();
        self.spells_api.set_script(params.clone(), air)?;
        // update init_data without affecting other keys
        self.spells_api.update_kv(params, json!(spell_distro.kv))?;

        // resubscribe spell
        if let Some(trigger_config) = trigger_config {
            let result = self
                .spell_event_bus_api
                .subscribe(spell_id.to_string(), trigger_config)
                .await;
            if let Err(err) = result {
                return Err(eyre!("{err}"));
            }
        }
        Ok(())
    }

    async fn remove_old_spell(&self, spell_name: &str, spell_id: &str) -> eyre::Result<()> {
        remove_spell(
            &get_deployer_particle_id(),
            &self.spell_storage,
            &self.services,
            &self.spell_event_bus_api,
            spell_id,
            self.root_worker_id,
        )
        .await
        .map_err(|err| {
            tracing::error!(spell_name, spell_id, "couldn't remove the old spell: {err}",);
            eyre!(err)
        })
    }

    async fn deploy_spell_common(&self, spell_distro: SpellDistro) -> eyre::Result<String> {
        let spell_id = install_spell(
            &self.services,
            &self.spell_storage,
            &self.spell_event_bus_api,
            &self.spells_api,
            self.root_worker_id,
            get_deployer_particle_id(),
            DEPLOYER_TTL,
            spell_distro.trigger_config,
            spell_distro.air.to_string(),
            json!(spell_distro.kv),
        )
        .await
        .map_err(|e| eyre!(e))?;
        self.services
            .add_alias(
                spell_distro.name.to_string(),
                self.root_worker_id,
                spell_id.clone(),
                self.management_id,
            )
            .await?;
        Ok(spell_id)
    }

    // Two spells are the same if they have the same alias
    fn find_same_spell(&self, new_spell: &SpellDistro) -> Option<SpellId> {
        let existing_spell =
            self.services
                .get_service_info("", self.root_worker_id, new_spell.name.to_string());
        match existing_spell {
            Err(ServiceError::NoSuchService(_)) => {
                log::debug!("no existing spell found for {}", new_spell.name);
                None
            }
            Err(err) => {
                log::error!(
                    "can't obtain details on a spell `{}` (will create a new one): {err}",
                    new_spell.name
                );
                None
            }
            Ok(spell) if spell.service_type != ServiceType::Spell => {
                log::warn!(
                "alias `{}` already used for a service [{}]; it will be used for a spell, the service won't be removed",
                new_spell.name,
                spell.id
            );
                None
            }
            Ok(spell) => Some(spell.id),
        }
    }

    async fn deploy_service_common(
        &self,
        service_distro: ServiceDistro,
    ) -> eyre::Result<ServiceStatus> {
        let service_name = service_distro.name.clone();
        let blueprint_id = self.add_modules(service_distro)?;

        match self.find_same_service(service_name.to_string(), &blueprint_id) {
            ServiceUpdateStatus::NeedUpdate(service_id) => {
                tracing::debug!(service_name, service_id, "found existing service that needs to be updated; will remove the old service and deploy a new one");
                let result = self.services.remove_service(
                    &get_deployer_particle_id(),
                    self.root_worker_id,
                    &service_id,
                    self.root_worker_id,
                    false,
                );
                if let Err(err) = result {
                    tracing::error!(
                        service_name, service_id,
                        "couldn't remove the old service (will install new service nevertheless): {err}",
                    );
                }
            }
            ServiceUpdateStatus::NoUpdate(service_id) => {
                tracing::debug!(
                    service_name,
                    service_id,
                    "found existing service that doesn't need to be updated; will skip update"
                );
                return Ok(ServiceStatus::Existing(service_id));
            }
            ServiceUpdateStatus::NotFound => {}
        }

        let service_id = self
            .services
            .create_service(
                ServiceType::Service,
                blueprint_id,
                self.root_worker_id,
                self.root_worker_id,
            )
            .await?;
        self.services
            .add_alias(
                service_name.to_string(),
                self.root_worker_id,
                service_id.clone(),
                self.management_id,
            )
            .await?;
        tracing::info!(service_name, service_id, "deployed a new service");
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
        let mut hashes = Vec::new();
        for config in service_distro.config.module {
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
            .add_blueprint(AddBlueprint::new(service_distro.name, hashes))?;
        Ok(blueprint_id)
    }
}

fn call_service(
    services: &ParticleAppServices,
    root_worker_id: PeerId,
    service_id: &str,
    function_name: &str,
    args: Vec<JValue>,
) -> eyre::Result<()> {
    let result = services.call_function(
        root_worker_id,
        service_id,
        function_name,
        args,
        None,
        root_worker_id,
        DEPLOYER_TTL,
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
                        Err(eyre!(
                            "Call {service_id}.{function_name} returned error: {}",
                            error
                        ))
                    } else {
                        Err(eyre!("Call {service_id}.{function_name} returned error"))
                    }
                } else {
                    Ok(())
                }
            };
            call_result.unwrap_or_else(|| {
                Err(eyre!(
                    "Call {service_id}.{function_name} return invalid result: {result}"
                ))
            })
        }
        FunctionOutcome::NotDefined { .. } => {
            Err(eyre!("Service {service_id} ({function_name}) not found"))
        }
        FunctionOutcome::Empty => Err(eyre!(
            "Call {service_id}.{function_name} didn't return any result"
        )),
        FunctionOutcome::Err(err) => Err(eyre!(err)),
    }
}
