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
use fluence_spell_dtos::value::{StringValue, UnitValue};
use serde_json::{json, Value as JValue, Value::Array};

use crate::utils::{parse_spell_id_from, process_func_outcome};
use fluence_spell_dtos::trigger_config::TriggerConfig;
use key_manager::KeyManager;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::ParticleAppServices;
use spell_event_bus::api::EventBusError;
use spell_event_bus::{api, api::SpellEventBusApi};
use spell_storage::SpellStorage;
use std::time::Duration;

pub(crate) async fn spell_install(
    sargs: Args,
    params: ParticleParams,
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = sargs.function_args.clone().into_iter();
    let script: String = Args::next("script", &mut args)?;
    let init_data: JValue = Args::next("data", &mut args)?;
    let user_config: TriggerConfig = Args::next("config", &mut args)?;
    let config = api::from_user_config(user_config.clone())?;

    let worker_id = if key_manager.is_host(params.host_id) {
        // direct hosting
        let deal_id = KeyManager::generate_deal_id(params.init_peer_id);
        match key_manager.get_worker_id(deal_id.clone()) {
            Ok(id) => id,
            Err(_) => key_manager.create_worker(Some(deal_id), params.init_peer_id)?,
        }
    } else {
        params.host_id
    };

    let spell_id = services.create_service(spell_storage.get_blueprint(), worker_id, worker_id)?;
    spell_storage.register_spell(spell_id.clone());

    // TODO: refactor these service calls
    // Save the script to the spell
    process_func_outcome::<UnitValue>(
        services.call_function(
            worker_id,
            &spell_id,
            "set_script_source_to_file",
            vec![json!(script)],
            None,
            worker_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_script_source_to_file",
    )?;

    // Save init_data to the spell's KV
    process_func_outcome::<UnitValue>(
        services.call_function(
            worker_id,
            &spell_id,
            "set_json_fields",
            vec![json!(init_data.to_string())],
            None,
            worker_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_json_fields",
    )?;

    // Save trigger config
    process_func_outcome::<UnitValue>(
        services.call_function(
            worker_id,
            &spell_id,
            "set_trigger_config",
            vec![json!(user_config)],
            None,
            worker_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_trigger_config",
    )?;

    if let Some(config) = config {
        // Scheduling the spell
        if let Err(err) = spell_event_bus_api
            .subscribe(spell_id.clone(), config.clone())
            .await
        {
            log::warn!("can't subscribe a spell {} to triggers {:?} via spell-event-bus-api: {}. Removing created spell service...", spell_id, config, err);

            spell_storage.unregister_spell(&spell_id);
            services.remove_service(params.host_id, spell_id, params.init_peer_id, true)?;

            return Err(JError::new(format!(
                "can't install a spell due to an internal error while subscribing to the triggers: {err}"
            )));
        }
    } else {
        log::trace!(
            "empty config given for spell {}, particle id {}",
            spell_id,
            params.id
        );
    }

    Ok(JValue::String(spell_id))
}

pub(crate) fn spell_list(spell_storage: SpellStorage) -> Result<JValue, JError> {
    Ok(Array(
        spell_storage
            .get_registered_spells()
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
    let spell_owner = services.get_service_owner(spell_id.clone(), params.host_id)?;
    let worker_creator = key_manager.get_worker_creator(worker_id)?;

    let is_spell_owner = init_peer_id == spell_owner;
    let is_worker_creator = init_peer_id == worker_creator;
    let is_worker = init_peer_id == worker_id;
    let is_management = key_manager.is_management(init_peer_id);

    if !is_spell_owner && !is_worker_creator && !is_worker && !is_management {
        return Err(JError::new(format!(
            "Failed to remove spell {spell_id}, spell can be removed by spell owner {spell_owner}, worker creator {worker_creator}, worker itself {worker_id} or peer manager"
        )));
    }

    if let Err(err) = spell_event_bus_api.unsubscribe(spell_id.clone()).await {
        log::warn!(
            "can't unsubscribe a spell {spell_id} from its triggers via spell-event-bus-api: {err}"
        );
        return Err(JError::new(format!(
            "can't remove a spell {spell_id} due to an internal error while unsubscribing from the triggers: {err}"
        )));
    }

    let spell_id = services.to_service_id(params.host_id, spell_id)?;
    spell_storage.unregister_spell(&spell_id);
    let owner_id = if is_worker_creator {
        worker_id
    } else {
        init_peer_id
    };
    services.remove_service(worker_id, spell_id, owner_id, true)?;
    Ok(())
}

pub(crate) async fn spell_update_config(
    args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    key_manager: KeyManager,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;

    let worker_id = params.host_id;
    let init_peer_id = params.init_peer_id;
    let spell_owner = services.get_service_owner(spell_id.clone(), worker_id)?;
    let worker_creator = key_manager.get_worker_creator(worker_id)?;

    let is_spell_owner = init_peer_id == spell_owner;
    let is_worker_creator = init_peer_id == worker_creator;
    let is_worker = init_peer_id == worker_id;
    let is_management = key_manager.is_management(init_peer_id);

    if !is_spell_owner && !is_worker_creator && !is_worker && !is_management {
        return Err(JError::new(format!(
            "Failed to update spell config {spell_id}, spell config can be updated by spell owner {spell_owner}, worker creator {worker_creator}, worker itself {worker_id} or peer manager; init_peer_id={init_peer_id}"
        )));
    }

    let user_config: TriggerConfig = Args::next("config", &mut args)?;
    let config = api::from_user_config(user_config.clone())?;

    process_func_outcome::<UnitValue>(
        services.call_function(
            worker_id,
            &spell_id,
            "set_trigger_config",
            vec![json!(user_config)],
            None,
            worker_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_trigger_config",
    )?;

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
        log::warn!("can't update a spell {spell_id} config via spell-event-bus-api: {err}");
        return Err(JError::new(format!(
            "can't update a spell {spell_id} config due to an internal error while updating the triggers: {err}"
        )));
    }

    Ok(())
}

pub(crate) fn get_spell_id(params: ParticleParams) -> Result<JValue, JError> {
    Ok(json!(parse_spell_id_from(&params.id)?))
}

pub(crate) fn get_spell_arg(
    args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
) -> Result<JValue, JError> {
    let spell_id = parse_spell_id_from(&params.id)?;
    let key = args.function_name;

    let str_value = process_func_outcome::<StringValue>(
        services.call_function(
            params.host_id,
            spell_id,
            "get_string",
            vec![json!(key)],
            Some(params.id.clone()),
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        spell_id,
        "get_string",
    )
    .map_err(|e| JError::new(f!("Failed to get argument {key} for spell {spell_id}: {e}")))?;

    serde_json::from_str(&str_value.str).map_err(Into::into)
}

pub(crate) fn store_error(
    mut args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
) -> Result<(), JError> {
    let spell_id = parse_spell_id_from(&params.id)?;

    args.function_args.push(json!(params.timestamp));
    process_func_outcome::<UnitValue>(
        services.call_function(
            params.host_id,
            spell_id,
            "store_error",
            args.function_args.clone(),
            Some(params.id.clone()),
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        spell_id,
        "store_error",
    )
    .map(drop)
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
    services: ParticleAppServices,
) -> Result<(), JError> {
    let spell_id = parse_spell_id_from(&params.id)?;
    let response: JValue = Args::next("spell_id", &mut args.function_args.into_iter())?;
    process_func_outcome::<UnitValue>(
        services.call_function(
            params.host_id,
            spell_id,
            "set_json_fields",
            vec![json!(response.to_string())],
            Some(params.id.clone()),
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        spell_id,
        "set_json_fields",
    )
    .map(drop)
    .map_err(|e| {
        JError::new(format!(
            "Failed to store response {response} for spell {spell_id}: {e}"
        ))
    })
}
