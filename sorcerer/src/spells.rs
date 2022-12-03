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
use std::time::Duration;

use fluence_spell_dtos::value::{StringValue, UnitValue};
use serde_json::{json, Value as JValue, Value::Array};

use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::ParticleAppServices;
use spell_event_bus::{
    api,
    api::{Spell, SpellEventBusApi, TimerConfig, PeerEventType},
};
use spell_storage::SpellStorage;
use crate::utils::{parse_spell_id_from, process_func_outcome};
use std::time::Duration;
use fluence_spell_dtos::trigger_config::{TriggerConfig};

/// Convert user-friendly config to event-bus-friendly config.
fn _from_user_config(user_config: TriggerConfig) -> Option<api::TriggersConfig> {
    let mut triggers = Vec::new();
    // Process timer config
    if user_config.clock.period_sec != 0 {
        triggers.push(api::TriggerConfig::Timer(TimerConfig {
            period: Duration::from_secs(user_config.clock.period_sec as u64),
        }));
    }

    // Process connections config
    let mut pool_events = Vec::with_capacity(2);
    if user_config.connections.connect {
        pool_events.push(PeerEventType::Connected);
        }
    if user_config.connections.disconnect {
        pool_events.push(PeerEventType::Disconnected);
        }
    if !pool_events.is_empty() {
        triggers.push(api::TriggerConfig::PeerEvent(api::PeerEventConfig {
            events: pool_events
        }));
    }

    if triggers.is_empty() {
        None
    } else {
        Some(api::TriggersConfig { triggers })
    }
}

pub(crate) fn spell_install(
    sargs: Args,
    params: ParticleParams,
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
) -> Result<JValue, JError> {
    let mut args = sargs.function_args.clone().into_iter();
    let script: String = Args::next("script", &mut args)?;
    let init_data: String = Args::next("data", &mut args)?;
    log::info!("Init data: {}", json!(init_data));
    // TODO: use TriggerConfig when it's deserializable
    let period: u64 = Args::next("period", &mut args)?;

    // TODO: create service on behalf of spell keypair
    let service_id = services.create_service(spell_storage.get_blueprint(), params.init_peer_id)?;
    spell_storage.register_spell(service_id.clone());

    // TODO: refactor these service calls
    // Save the script to the spell
    process_func_outcome::<UnitValue>(
        services.call_function(
            &service_id,
            "set_script_source_to_file",
            vec![json!(script)],
            None,
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &service_id,
        "set_script_source_to_file",
    )?;

    // Save init_data to the spell's KV
    process_func_outcome::<UnitValue>(
        services.call_function(
            &service_id,
            "set_json_fields",
            vec![json!(init_data)],
            None,
            params.init_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        &service_id,
        "set_json_fields",
    )?;
    // TODO: also save trigger config

    // Scheduling the spell
    let config = api::TriggersConfig {
        triggers: vec![api::TriggerConfig::Timer(TimerConfig {
            period: Duration::from_secs(period),
        })],
    };
    spell_event_bus_api.subscribe(
        Spell {
            id: service_id.clone(),
        },
        config,
    )?;
    Ok(JValue::String(service_id))
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
    spell_event_bus_api: SpellEventBusApi,
    args: Args,
    params: ParticleParams,
    spell_storage: SpellStorage,
    services: ParticleAppServices,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;

    spell_event_bus_api.unsubscribe(spell_id.clone()).await?;

    // TODO: remove spells by aliases too
    spell_storage.unregister_spell(&spell_id);
    services.remove_service(spell_id, params.init_peer_id)?;
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
