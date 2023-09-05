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
use fluence_libp2p::PeerId;
use serde_json::Value as JValue;
use std::str::FromStr;

use crate::spell_builtins::remove_spell;
use key_manager::KeyManager;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::ParticleAppServices;
use spell_event_bus::api::SpellEventBusApi;
use spell_storage::SpellStorage;

pub(crate) fn create_worker(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: Option<String> = Args::next_opt("deal_id", &mut args)?;
    Ok(JValue::String(
        key_manager
            .create_worker(deal_id, params.init_peer_id)?
            .to_base58(),
    ))
}

pub(crate) fn get_worker_peer_id(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: Option<String> = Args::next_opt("deal_id", &mut args)?;

    Ok(JValue::String(
        key_manager
            .get_worker_id(deal_id, params.init_peer_id)?
            .to_base58(),
    ))
}

pub(crate) fn get_worker_peer_id_opt(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: Option<String> = Args::next_opt("deal_id", &mut args)?;

    Ok(JValue::Array(
        key_manager
            .get_worker_id(deal_id, params.init_peer_id)
            .map(|id| vec![JValue::String(id.to_base58())])
            .unwrap_or_default(),
    ))
}

pub(crate) async fn remove_worker(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
    services: ParticleAppServices,
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let worker_id: String = Args::next("worker_id", &mut args)?;
    let worker_id = PeerId::from_str(&worker_id)?;
    let worker_creator = key_manager.get_worker_creator(worker_id)?;

    if params.init_peer_id != worker_creator && params.init_peer_id != worker_id {
        return Err(JError::new(format!("Worker {worker_id} can be removed only by worker creator {worker_creator} or worker itself")));
    }

    key_manager.remove_worker(worker_id)?;

    let spells: Vec<_> = spell_storage
        .get_registered_spells()
        .get(&worker_id)
        .cloned()
        .unwrap_or_default();

    for s in spells {
        remove_spell(
            &params.id,
            &spell_storage,
            &services,
            &spell_event_bus_api,
            &s,
            worker_id,
        )
        .await
        .map_err(|e| {
            JError::new(format!(
                "Worker removing failed due to spell removing failure: {e}"
            ))
        })?;
    }

    services.remove_services(worker_id)?;
    Ok(())
}

pub(crate) fn worker_list(key_manager: KeyManager) -> Result<JValue, JError> {
    Ok(JValue::Array(
        key_manager
            .list_workers()
            .into_iter()
            .map(|p| JValue::String(p.to_base58()))
            .collect(),
    ))
}
