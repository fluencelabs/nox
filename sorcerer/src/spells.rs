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

use now_millis::now_ms;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::ParticleAppServices;
use serde_json::json;
use serde_json::Value as JValue;
use serde_json::Value::Array;
use spell_event_bus::scheduler::api::{SchedulerApi, TimerConfig};
use spell_storage::SpellStorage;
use std::time::Duration;
use uuid_utils::uuid;

pub(crate) fn spell_install(
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    spell_scheduler_api: SchedulerApi,
    sargs: Args,
    params: ParticleParams,
) -> Result<JValue, JError> {
    let mut args = sargs.function_args.clone().into_iter();
    let script: String = Args::next("script", &mut args)?;
    // TODO: redo config when other event are supported
    let period: u64 = Args::next("period", &mut args)?;

    let service_id = services.create_service(spell_storage.get_blueprint(), params.init_peer_id)?;
    spell_storage.register_spell(service_id.clone());

    // Save the script to the spell.
    let script_arg = JValue::String(script);
    let script_tetraplet = sargs.tetraplets[0].clone();
    let spell_args = Args {
        service_id: service_id.clone(),
        function_name: "set_script_source_to_file".to_string(),
        function_args: vec![script_arg],
        tetraplets: vec![script_tetraplet],
    };
    let particle = ParticleParams {
        id: uuid(),
        init_peer_id: params.init_peer_id,
        timestamp: now_ms() as u64,
        ttl: params.ttl,
        script: "".to_string(),
        signature: vec![],
    };
    services.call_service(spell_args, particle);
    // TODO: also save config

    // Scheduling the spell
    spell_scheduler_api.add(
        service_id.clone(),
        TimerConfig {
            period: Duration::from_secs(period),
        },
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
pub(crate) fn spell_remove(
    spell_storage: SpellStorage,
    services: ParticleAppServices,
    args: Args,
    params: ParticleParams,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let spell_id: String = Args::next("spell_id", &mut args)?;

    spell_storage.unregister_spell(&spell_id);
    services.remove_service(spell_id, params.init_peer_id)?;
    Ok(())
}

pub(crate) fn get_spell_id(_args: Args, params: ParticleParams) -> Result<JValue, JError> {
    if params.id.starts_with("spell_") {
        let spell_id = params
            .id
            .split('_')
            .collect::<Vec<&str>>()
            .get(1)
            .ok_or(JError(json!("Invalid particle id format")))?
            .to_string();
        Ok(json!(spell_id))
    } else {
        Err(JError(json!("Invalid particle id format")))
    }
}
