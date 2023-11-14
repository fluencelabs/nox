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
use fluence_spell_dtos::trigger_config::TriggerConfig;
use futures::TryFutureExt;
use serde_json::Value as JValue;
use std::str::FromStr;
use std::time::Duration;

use crate::spell_builtins::remove_spell;
use key_manager::KeyManager;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::ParticleAppServices;
use spell_event_bus::api::{from_user_config, SpellEventBusApi};
use spell_service_api::{CallParams, SpellServiceApi};
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

    let spells: Vec<_> = spell_storage.get_registered_spells_by(worker_id);

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

pub(crate) async fn deactivate_deal(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("worker_id", &mut args)?;

    if !key_manager.is_management(params.init_peer_id) && !key_manager.is_host(params.init_peer_id)
    {
        return Err(JError::new(format!(
            "Only management or host peer can deactivate deal"
        )));
    }

    let worker_id = key_manager.get_worker_id(Some(deal_id), params.init_peer_id)?;

    if !key_manager.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been deactivated"));
    }

    let spells = spell_storage.get_registered_spells_by(worker_id);

    for spell_id in spells.into_iter() {
        spell_event_bus_api
            .unsubscribe(spell_id.clone())
            .map_err(|e| {
                JError::new(format!(
                    "Deal deactivation failed due to failure to stop spell {spell_id} : {e}"
                ))
            })
            .await?;

        spell_service_api
            .set_trigger_config(
                CallParams::new(
                    worker_id,
                    worker_id,
                    spell_id.clone(),
                    None,
                    Duration::from_millis(params.ttl as u64),
                ),
                TriggerConfig::default(),
            )
            .map_err(|e| {
                JError::new(format!(
                    "Deal deactivation failed due to failure to stop spell {spell_id} : {e}"
                ))
            })?;
    }

    key_manager.deactivate_worker(worker_id)?;

    Ok(())
}

pub(crate) async fn activate_deal(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("worker_id", &mut args)?;

    if !key_manager.is_management(params.init_peer_id) && !key_manager.is_host(params.init_peer_id)
    {
        return Err(JError::new(format!(
            "Only management or host peer can activate deal"
        )));
    }

    let worker_id = key_manager.get_worker_id(Some(deal_id), params.init_peer_id)?;

    if key_manager.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been activated"));
    }

    let installation_spell_id =
        services.resolve_alias(&params.id, worker_id, "worker-spell".to_string())?;

    let trigger_config = spell_service_api.get_trigger_config(CallParams::new(
        params.init_peer_id,
        key_manager.get_host_peer_id(),
        "decider".to_string(),
        None,
        Duration::from_millis(params.ttl as u64),
    ))?;

    spell_service_api.set_trigger_config(
        CallParams::new(
            worker_id,
            worker_id,
            installation_spell_id.clone(),
            None,
            Duration::from_millis(params.ttl as u64),
        ),
        trigger_config.clone(),
    )?;

    let trigger_config = from_user_config(&trigger_config)?.ok_or(JError::new(format!(
        "Deal activation failed due to failure to parse trigger config"
    )))?;

    spell_event_bus_api
        .subscribe(installation_spell_id, trigger_config)
        .map_err(|e| {
            JError::new(format!(
                "Deal activation failed due to failure to start worker spell : {e}"
            ))
        })
        .await?;

    key_manager.activate_worker(worker_id)?;
    Ok(())
}

pub(crate) fn is_deal_active(
    args: Args,
    params: ParticleParams,
    key_manager: KeyManager,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("worker_id", &mut args)?;
    let worker_id = key_manager.get_worker_id(Some(deal_id), params.init_peer_id)?;
    Ok(JValue::Bool(key_manager.is_worker_active(worker_id)))
}
