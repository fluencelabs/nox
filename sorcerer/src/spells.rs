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
    let init_data: String = Args::next("data", &mut args)?;
    log::info!("Init data: {}", json!(init_data));
    let user_config: TriggerConfig = Args::next("config", &mut args)?;
    let config = api::from_user_config(user_config.clone())?;

    let spell_peer_id = key_manager
        .get_or_generate_keypair(&params.init_peer_id.to_base58())
        .map_err(|e| JError::new(f!("{e}")))?
        .get_peer_id();

    let spell_id = services.create_service(spell_storage.get_blueprint(), spell_peer_id)?;
    spell_storage.register_spell(spell_id.clone());

    // TODO: refactor these service calls
    // Save the script to the spell
    process_func_outcome::<UnitValue>(
        services.call_function(
            &spell_id,
            "set_script_source_to_file",
            vec![json!(script)],
            None,
            spell_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_script_source_to_file",
    )?;

    // Save init_data to the spell's KV
    process_func_outcome::<UnitValue>(
        services.call_function(
            &spell_id,
            "set_json_fields",
            vec![json!(init_data)],
            None,
            spell_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_json_fields",
    )?;

    // Save trigger config
    process_func_outcome::<UnitValue>(
        services.call_function(
            &spell_id,
            "set_trigger_config",
            vec![json!(user_config)],
            None,
            spell_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &spell_id,
        "set_trigger_config",
    )?;

    // Scheduling the spell
    if let Err(err) = spell_event_bus_api
        .subscribe(spell_id.clone(), config.clone())
        .await
    {
        log::warn!("can't subscribe a spell {} to triggers {:?} via spell-event-bus-api: {}. Removing created spell service...", spell_id, config, err);

        spell_storage.unregister_spell(&spell_id);
        services.remove_service(spell_id, spell_peer_id, true)?;

        return Err(JError::new(format!(
            "can't install a spell due to an internal error while subscribing to the triggers: {}",
            err
        )));
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
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;
    if let Err(err) = spell_event_bus_api.unsubscribe(spell_id.clone()).await {
        log::warn!(
            "can't unsubscribe a spell {} from its triggers via spell-event-bus-api: {}",
            spell_id,
            err
        );
        return Err(JError::new(format!(
            "can't remove a spell {} due to an internal error while unsubscribing from the triggers: {}",
            spell_id,
            err
        )));
    }

    // TODO: remove spells by aliases too
    spell_storage.unregister_spell(&spell_id);
    services.remove_service(spell_id, params.init_peer_id, true)?;
    Ok(())
}

pub(crate) fn get_spell_id(_args: Args, params: ParticleParams) -> Result<JValue, JError> {
    Ok(json!(parse_spell_id_from(&params.id)?))
}

pub(crate) fn get_spell_arg(
    args: Args,
    params: ParticleParams,
    services: ParticleAppServices,
) -> Result<JValue, JError> {
    let spell_id = parse_spell_id_from(&params.id)?;
    let key = args.function_name;

    process_func_outcome::<StringValue>(
        services.call_function(
            spell_id,
            "get_string",
            vec![json!(key.clone())],
            Some(params.id.clone()),
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        spell_id,
        "get_string",
    )
    .map(|v| json!(v.str))
    .map_err(|e| JError::new(f!("Failed to get argument {key} for spell {spell_id}: {e}")))
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

    process_func_outcome::<UnitValue>(
        services.call_function(
            spell_id,
            "set_json_fields",
            args.function_args.clone(),
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
            "Failed to store response {:?} for spell {}: {}",
            args.function_args, spell_id, e
        ))
    })
}
