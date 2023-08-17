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
//use fluence_spell_dtos::value::{StringValue, UnitValue};
use serde_json::{json, Value as JValue, Value, Value::Array};

use crate::utils::parse_spell_id_from;
use fluence_spell_dtos::trigger_config::TriggerConfig;
use key_manager::KeyManager;
use libp2p::PeerId;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::{ParticleAppServices, ServiceType};
use spell_event_bus::api::EventBusError;
use spell_event_bus::{api, api::SpellEventBusApi};
use spell_service_api::{CallParams, SpellServiceApi};
use spell_storage::SpellStorage;
use std::time::Duration;

pub async fn remove_spell(
    particle_id: &str,
    spell_storage: &SpellStorage,
    services: &ParticleAppServices,
    spell_event_bus_api: &SpellEventBusApi,
    spell_id: String,
    worker_id: PeerId,
) -> Result<(), JError> {
    if let Err(err) = spell_event_bus_api.unsubscribe(spell_id.clone()).await {
        log::warn!(
            "can't unsubscribe a spell {spell_id} from its triggers via spell-event-bus-api: {err}"
        );
        return Err(JError::new(format!(
            "can't remove a spell {spell_id} due to an internal error while unsubscribing from the triggers: {err}"
        )));
    }

    spell_storage.unregister_spell(worker_id, &spell_id);
    services.remove_service(particle_id, worker_id, spell_id, worker_id, true)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn install_spell(
    services: &ParticleAppServices,
    spell_storage: &SpellStorage,
    spell_event_bus_api: &SpellEventBusApi,
    spell_service_api: &SpellServiceApi,
    worker_id: PeerId,
    particle_id: String,
    ttl: u64,
    user_config: TriggerConfig,
    script: String,
    init_data: Value,
) -> Result<String, JError> {
    let config = api::from_user_config(&user_config)?;

    let spell_id = services.create_service(
        ServiceType::Spell,
        spell_storage.get_blueprint(),
        worker_id,
        worker_id,
    )?;
    spell_storage.register_spell(worker_id, spell_id.clone());

    let params = CallParams::local(spell_id.clone(), worker_id, Duration::from_millis(ttl));
    // TODO: refactor these service calls
    // Save the script to the spell
    spell_service_api.set_script(params.clone(), script)?;
    // Save init_data to the spell's KV
    spell_service_api.update_kv(params.clone(), init_data)?;
    // Save trigger config
    spell_service_api.set_trigger_config(params, user_config)?;

    if let Some(config) = config {
        // Scheduling the spell
        if let Err(err) = spell_event_bus_api
            .subscribe(spell_id.clone(), config.clone())
            .await
        {
            log::warn!("can't subscribe a spell {} to triggers {:?} via spell-event-bus-api: {}. Removing created spell service...", spell_id, config, err);

            spell_storage.unregister_spell(worker_id, &spell_id);
            services.remove_service(&particle_id, worker_id, spell_id, worker_id, true)?;

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
    worker_id: PeerId,
    ttl: u64,
    spell_id: String,
) -> Result<SpellInfo, JError> {
    let params = CallParams::local(spell_id.clone(), worker_id, Duration::from_millis(ttl));
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
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = sargs.function_args.clone().into_iter();
    let script: String = Args::next("script", &mut args)?;
    let init_data: JValue = Args::next("data", &mut args)?;
    let trigger_config: TriggerConfig = Args::next("trigger_config", &mut args)?;
    let init_peer_id = params.init_peer_id;

    let is_management = key_manager.is_management(init_peer_id);
    if key_manager.is_host(params.host_id) && !is_management {
        return Err(JError::new("Failed to install spell in the root scope, only management peer id can install top-level spells"));
    }

    let worker_id = params.host_id;
    let worker_creator = key_manager.get_worker_creator(params.host_id)?;

    let is_worker = init_peer_id == worker_id;
    let is_worker_creator = init_peer_id == worker_creator;
    if !is_management && !is_worker && !is_worker_creator {
        return Err(JError::new(format!("Failed to install spell on {worker_id}, spell can be installed by worker creator {worker_creator}, worker itself {worker_id} or peer manager; init_peer_id={init_peer_id}")));
    }

    let spell_id = install_spell(
        &services,
        &spell_storage,
        &spell_event_bus_api,
        &spell_service_api,
        worker_id,
        params.id,
        params.ttl as u64,
        trigger_config,
        script,
        init_data,
    )
    .await?;
    Ok(JValue::String(spell_id))
}

pub(crate) fn spell_list(
    params: ParticleParams,
    spell_storage: SpellStorage,
) -> Result<JValue, JError> {
    Ok(Array(
        spell_storage
            .get_registered_spells()
            .get(&params.host_id)
            .cloned()
            .unwrap_or_default()
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
    key_manager: KeyManager,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;

    let worker_id = params.host_id;
    let init_peer_id = params.init_peer_id;
    let worker_creator = key_manager.get_worker_creator(worker_id)?;

    let is_worker_creator = init_peer_id == worker_creator;
    let is_worker = init_peer_id == worker_id;
    let is_management = key_manager.is_management(init_peer_id);

    if !is_worker_creator && !is_worker && !is_management {
        return Err(JError::new(format!(
            "Failed to remove spell {spell_id}, spell can be removed by worker creator {worker_creator}, worker itself {worker_id} or peer manager"
        )));
    }

    let spell_id = services.to_service_id(&params.id, params.host_id, spell_id)?;

    remove_spell(
        &params.id,
        &spell_storage,
        &services,
        &spell_event_bus_api,
        spell_id,
        worker_id,
    )
    .await
}

pub(crate) async fn spell_update_config(
    args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
    key_manager: KeyManager,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id_or_alias: String = Args::next("spell_id", &mut args)?;

    let worker_id = params.host_id;
    let init_peer_id = params.init_peer_id;
    let worker_creator = key_manager.get_worker_creator(worker_id)?;

    let is_worker_creator = init_peer_id == worker_creator;
    let is_worker = init_peer_id == worker_id;
    let is_management = key_manager.is_management(init_peer_id);

    if !is_worker_creator && !is_worker && !is_management {
        return Err(JError::new(format!(
            "Failed to update spell config {spell_id_or_alias}, spell config can be updated by worker creator {worker_creator}, worker itself {worker_id} or peer manager; init_peer_id={init_peer_id}"
        )));
    }

    let spell_id = services.to_service_id(&params.id, worker_id, spell_id_or_alias.clone())?;

    let user_config: TriggerConfig = Args::next("config", &mut args)?;
    let config = api::from_user_config(&user_config)?;

    let params = CallParams::local(
        spell_id.clone(),
        worker_id,
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

    let str_value: String = spell_service_api
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
    // services: ParticleAppServices,
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
    let response: JValue = Args::next("spell_id", &mut args.function_args.into_iter())?;
    let call_params = CallParams::from(spell_id.clone(), params);
    spell_service_api
        .update_kv(call_params, response.clone())
        .map_err(|err| {
            JError::new(format!(
                "Failed to store response {response} for spell {spell_id}: {err}"
            ))
        })
}
