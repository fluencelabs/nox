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
use serde_json::{json, Value as JValue, Value, Value::Array};
use std::sync::Arc;

use crate::utils::parse_spell_id_from;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use libp2p::PeerId;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::{ParticleAppServices, PeerScope, ServiceType};
use spell_event_bus::api::EventBusError;
use spell_event_bus::{api, api::SpellEventBusApi};
use spell_service_api::{CallParams, SpellServiceApi};
use spell_storage::SpellStorage;
use std::time::Duration;
use workers::{PeerScopes, WorkersOperations};

pub async fn remove_spell(
    particle_id: &str,
    spell_storage: &SpellStorage,
    services: &ParticleAppServices,
    spell_event_bus_api: &SpellEventBusApi,
    spell_id: &str,
    peer_scope: PeerScope,
    init_peer_id: PeerId,
) -> Result<(), JError> {
    if let Err(err) = spell_event_bus_api.unsubscribe(spell_id.to_string()).await {
        log::warn!(
            "can't unsubscribe a spell {spell_id} from its triggers via spell-event-bus-api: {err}"
        );
        return Err(JError::new(format!(
            "can't remove a spell {spell_id} due to an internal error while unsubscribing from the triggers: {err}"
        )));
    }

    spell_storage.unregister_spell(peer_scope, spell_id);
    services
        .remove_service(peer_scope, particle_id, spell_id, init_peer_id, true)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn install_spell(
    services: &ParticleAppServices,
    spell_storage: &SpellStorage,
    spell_event_bus_api: &SpellEventBusApi,
    spell_service_api: &SpellServiceApi,
    peer_scope: PeerScope,
    particle_id: String,
    ttl: Duration,
    user_config: TriggerConfig,
    script: String,
    init_data: Value,
    owner_id: PeerId,
) -> Result<String, JError> {
    let config = api::from_user_config(&user_config)?;

    let spell_id = services
        .create_service(
            peer_scope,
            ServiceType::Spell,
            spell_storage.get_blueprint(),
            owner_id,
        )
        .await?;
    spell_storage.register_spell(peer_scope, spell_id.clone());

    let params = CallParams::local(peer_scope, spell_id.clone(), owner_id, ttl);
    // Save the script to the spell
    spell_service_api.set_script(params.clone(), script)?;
    // Save init_data to the spell's KV
    let self_particle_id = format!("spell_{spell_id}_0");
    let init_kv_params = CallParams::new(
        owner_id,
        peer_scope,
        spell_id.clone(),
        Some(self_particle_id),
        ttl,
    );
    spell_service_api.update_kv(init_kv_params.clone(), init_data)?;
    // Save trigger config
    spell_service_api.set_trigger_config(params, user_config)?;

    if let Some(config) = config {
        // Scheduling the spell
        if let Err(err) = spell_event_bus_api
            .subscribe(spell_id.clone(), config.clone())
            .await
        {
            log::warn!("can't subscribe a spell {} to triggers {:?} via spell-event-bus-api: {}. Removing created spell service...", spell_id, config, err);

            spell_storage.unregister_spell(peer_scope, &spell_id);
            services
                .remove_service(peer_scope, &particle_id, &spell_id, owner_id, true)
                .await?;

            return Err(JError::new(format!(
                "can't install a spell due to an internal error while subscribing to the triggers: {err}"
            )));
        }
    } else {
        tracing::trace!(
            particle_id = particle_id,
            "empty config given for spell {}",
            spell_id
        );
    }

    Ok(spell_id)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpellInfo {
    pub script: String,
    pub trigger_config: TriggerConfig,
}

pub fn get_spell_info(
    spell_service_api: &SpellServiceApi,
    peer_scope: PeerScope,
    ttl: Duration,
    spell_id: String,
    init_peer_id: PeerId,
) -> Result<SpellInfo, JError> {
    let params = CallParams::local(peer_scope, spell_id.clone(), init_peer_id, ttl);
    let trigger_config = spell_service_api
        .get_trigger_config(params.clone())
        .map_err(|e| JError::new(f!("Failed to get trigger_config for spell {spell_id}: {e}")))?;

    let script = spell_service_api
        .get_script(params)
        .map_err(|e| JError::new(f!("Failed to get trigger_config for spell {spell_id}: {e}")))?;
    Ok(SpellInfo {
        script,
        trigger_config,
    })
}

pub(crate) async fn spell_install(
    sargs: Args,
    params: ParticleParams,
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
    workers: Arc<dyn WorkersOperations>,
    scopes: PeerScopes,
) -> Result<JValue, JError> {
    let mut args = sargs.function_args.clone().into_iter();
    let script: String = Args::next("script", &mut args)?;
    let init_data: JValue = Args::next("data", &mut args)?;
    let trigger_config: TriggerConfig = Args::next("trigger_config", &mut args)?;
    let alias: Option<String> = Args::next_opt("alias", &mut args)?;

    let init_peer_id = params.init_peer_id;

    let is_management = scopes.is_management(init_peer_id);

    let owner_id = match params.peer_scope {
        PeerScope::WorkerId(worker_id) => {
            let is_worker = init_peer_id == worker_id.into();
            let worker_creator = workers.get_worker_creator(worker_id)?;
            let is_worker_creator = init_peer_id == worker_creator;
            if !is_management && !is_worker && !is_worker_creator {
                return Err(JError::new(format!("Failed to install spell on {worker_id:?}, spell can be installed by worker creator {worker_creator}, worker itself {worker_id} or peer manager; init_peer_id={init_peer_id}")));
            }
            worker_id.into()
        }
        PeerScope::Host => {
            if !is_management {
                return Err(JError::new("Failed to install spell in the root scope, only management peer id can install top-level spells"));
            }
            scopes.get_host_peer_id()
        }
    };

    let spell_id = install_spell(
        &services,
        &spell_storage,
        &spell_event_bus_api,
        &spell_service_api,
        params.peer_scope,
        params.id.clone(),
        Duration::from_millis(params.ttl as u64),
        trigger_config,
        script,
        init_data,
        owner_id,
    )
    .await?;

    if let Some(alias) = alias {
        if let Err(e) = services
            .add_alias(
                params.peer_scope,
                alias.clone(),
                spell_id.clone(),
                init_peer_id,
            )
            .await
        {
            // Remove the spell if we failed to add an alias
            remove_spell(
                &params.id,
                &spell_storage,
                &services,
                &spell_event_bus_api,
                &spell_id,
                params.peer_scope,
                init_peer_id,
            )
            .await?;

            return Err(JError::new(format!(
                "Failed to add alias {} for spell {}: {:?}",
                alias, spell_id, e
            )));
        }
    }

    Ok(JValue::String(spell_id))
}

pub(crate) fn spell_list(
    params: ParticleParams,
    spell_storage: SpellStorage,
) -> Result<JValue, JError> {
    Ok(Array(
        spell_storage
            .get_registered_spells_by(params.peer_scope)
            .into_iter()
            .map(JValue::String)
            .collect(),
    ))
}

pub(crate) async fn spell_remove(
    args: Args,
    params: ParticleParams,
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    workers: Arc<dyn WorkersOperations>,
    scopes: PeerScopes,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;

    let peer_scope = params.peer_scope;
    let init_peer_id = params.init_peer_id;

    let owner_peer_id: PeerId = match peer_scope {
        PeerScope::WorkerId(worker_id) => {
            let worker_creator = workers.get_worker_creator(worker_id)?;
            let is_worker_creator = init_peer_id == worker_creator;
            let is_worker = init_peer_id == worker_id.into();
            let is_management = scopes.is_management(init_peer_id);
            if !is_worker_creator && !is_worker && !is_management {
                return Err(JError::new(format!(
                    "Failed to remove spell {spell_id}, spell can be removed by worker creator {worker_creator}, worker itself {worker_id} or peer manager"
                )));
            }
            worker_id.into()
        }
        PeerScope::Host => {
            let host_peer_id = scopes.get_host_peer_id();
            let is_host = init_peer_id == host_peer_id;
            let is_management = scopes.is_management(init_peer_id);
            if !is_host && !is_management {
                return Err(JError::new(format!(
                    "Failed to remove spell {spell_id}, worker itself {host_peer_id} or peer manager"
                )));
            }
            host_peer_id
        }
    };

    let spell_id = services.to_service_id(params.peer_scope, spell_id, &params.id)?;

    remove_spell(
        &params.id,
        &spell_storage,
        &services,
        &spell_event_bus_api,
        &spell_id,
        peer_scope,
        owner_peer_id,
    )
    .await
}

pub(crate) async fn spell_update_config(
    args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
    workers: Arc<dyn WorkersOperations>,
    scopes: PeerScopes,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id_or_alias: String = Args::next("spell_id", &mut args)?;

    let peer_scope = params.peer_scope;
    let init_peer_id = params.init_peer_id;

    match peer_scope {
        PeerScope::WorkerId(worker_id) => {
            let worker_creator = workers.get_worker_creator(worker_id)?;
            let is_worker_creator = init_peer_id == worker_creator;
            let is_worker = init_peer_id == worker_id.into();
            let is_management = scopes.is_management(init_peer_id);
            if !is_worker_creator && !is_worker && !is_management {
                return Err(JError::new(format!(
                    "Failed to update spell config {spell_id_or_alias}, spell config can be updated by worker creator {worker_creator}, worker itself {worker_id} or peer manager; init_peer_id={init_peer_id}"
                )));
            }
        }
        PeerScope::Host => {
            let host_peer_id = scopes.get_host_peer_id();
            let is_host = init_peer_id == host_peer_id;
            let is_management = scopes.is_management(init_peer_id);
            if !is_host && !is_management {
                return Err(JError::new(format!(
                    "Failed to update spell config {spell_id_or_alias}, spell config can be updated by worker itself {host_peer_id} or peer manager; init_peer_id={init_peer_id}"
                )));
            }
        }
    }

    let spell_id = services.to_service_id(peer_scope, spell_id_or_alias.clone(), &params.id)?;

    let user_config: TriggerConfig = Args::next("config", &mut args)?;
    let config = api::from_user_config(&user_config)?;
    let init_peer_id = scopes.to_peer_id(peer_scope);
    let params = CallParams::local(
        peer_scope,
        spell_id.clone(),
        init_peer_id,
        Duration::from_millis(params.ttl as u64),
    );
    spell_service_api.set_trigger_config(params, user_config)?;

    let result: Result<(), EventBusError> = try {
        // we unsubscribe the spell from the current config anyway
        spell_event_bus_api.unsubscribe(spell_id.clone()).await?;
        if let Some(config) = config {
            // and if the config isn't empty, we subscribe it to the new one
            spell_event_bus_api
                .subscribe(spell_id.clone(), config.clone())
                .await?;
        }
    };

    if let Err(err) = result {
        log::warn!(
            "can't update a spell {spell_id_or_alias} config via spell-event-bus-api: {err}"
        );
        return Err(JError::new(format!(
            "can't update a spell {spell_id_or_alias} config due to an internal error while updating the triggers: {err}"
        )));
    }

    Ok(())
}

pub(crate) fn get_spell_id(params: ParticleParams) -> Result<JValue, JError> {
    Ok(json!(parse_spell_id_from(&params)?))
}

pub(crate) fn get_spell_arg(
    args: Args,
    params: ParticleParams,
    spell_service_api: SpellServiceApi,
) -> Result<JValue, JError> {
    let spell_id = parse_spell_id_from(&params)?;
    let key = args.function_name;
    let call_params = CallParams::from(spell_id.clone(), params);

    let str_value = spell_service_api
        .get_string(call_params, key.clone())
        .map_err(|e| JError::new(f!("Failed to get argument {key} for spell {spell_id}: {e}")))
        .and_then(|value| value.ok_or_else(|| JError::new("value not found")))?;

    serde_json::from_str(&str_value).map_err(|e| {
        JError::new(f!(
            "Failed to parse argument `{key} -> {str_value}` for spell {spell_id}: {e}"
        ))
    })
}

pub(crate) fn store_error(
    mut args: Args,
    params: ParticleParams,
    spell_service_api: SpellServiceApi,
) -> Result<(), JError> {
    let spell_id = parse_spell_id_from(&params)?;

    args.function_args.push(json!(params.timestamp));
    let call_params = CallParams::from(spell_id.clone(), params);
    spell_service_api
        .store_error(call_params, args.function_args.clone())
        .map_err(|e| {
            JError::new(format!(
                "Failed to store error {:?} for spell {}: {}",
                args.function_args, spell_id, e
            ))
        })
}

pub(crate) fn store_response(
    args: Args,
    params: ParticleParams,
    spell_service_api: SpellServiceApi,
) -> Result<(), JError> {
    let spell_id = parse_spell_id_from(&params)?;
    let response: Option<JValue> = Args::next_opt("response", &mut args.function_args.into_iter())?;

    if let Some(response) = response {
        let call_params = CallParams::from(spell_id.clone(), params);
        spell_service_api
            .update_kv(call_params, response.clone())
            .map_err(|err| {
                JError::new(format!(
                    "Failed to store response {response} for spell {spell_id}: {err}"
                ))
            })
    } else {
        Ok(())
    }
}
