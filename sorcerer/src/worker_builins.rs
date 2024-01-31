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
use std::sync::Arc;
use std::time::Duration;

use crate::spell_builtins::remove_spell;
use particle_args::{Args, JError};
use particle_execution::ParticleParams;
use particle_services::{ParticleAppServices, PeerScope};
use spell_event_bus::api::{from_user_config, SpellEventBusApi};
use spell_service_api::{CallParams, SpellServiceApi};
use spell_storage::SpellStorage;
use workers::{PeerScopes, WorkerParams, Workers};

pub(crate) async fn create_worker(
    args: Args,
    params: ParticleParams,
    workers: Arc<Workers>,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;
    let cu_count: usize = Args::next_opt("cu_count", &mut args)?.unwrap_or(1);
    Ok(JValue::String(
        workers
            .create_worker(WorkerParams::new(deal_id, params.init_peer_id, cu_count))
            .await?
            .to_string(),
    ))
}

pub(crate) fn get_worker_peer_id(args: Args, workers: Arc<Workers>) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;

    Ok(JValue::Array(
        workers
            .get_worker_id(deal_id)
            .map(|id| vec![JValue::String(id.to_string())])
            .unwrap_or_default(),
    ))
}

pub(crate) async fn remove_worker(
    args: Args,
    params: ParticleParams,
    workers: Arc<Workers>,
    services: ParticleAppServices,
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
    scopes: PeerScopes,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let worker_id: String = Args::next("worker_id", &mut args)?;
    let worker_peer_id = PeerId::from_str(&worker_id)?;
    let peer_scope = scopes
        .scope(worker_peer_id)
        .map_err(|_| JError::new(format!("Worker {worker_id} not found")))?;

    match peer_scope {
        PeerScope::WorkerId(worker_id) => {
            let worker_creator = workers.get_worker_creator(worker_id)?;
            if params.init_peer_id != worker_creator && params.init_peer_id != worker_peer_id {
                return Err(JError::new(format!("Worker {worker_id} can be removed only by worker creator {worker_creator} or worker itself")));
            }
            workers.remove_worker(worker_id).await?;
            let spells: Vec<_> = spell_storage.get_registered_spells_by(peer_scope);
            for s in spells {
                remove_spell(
                    &params.id,
                    &spell_storage,
                    &services,
                    &spell_event_bus_api,
                    &s,
                    peer_scope,
                    worker_id.into(),
                )
                .map_err(|e| {
                    JError::new(format!(
                        "Worker removing failed due to spell removing failure: {e}"
                    ))
                })
                .await?;
            }
            services.remove_services(peer_scope).await?;
        }
        PeerScope::Host => return Err(JError::new(format!("Worker {worker_id} can be removed"))),
    };

    Ok(())
}

pub(crate) fn worker_list(workers: Arc<Workers>) -> Result<JValue, JError> {
    Ok(JValue::Array(
        workers
            .list_workers()
            .into_iter()
            .map(|p| JValue::String(p.to_string()))
            .collect(),
    ))
}

pub(crate) async fn deactivate_deal(
    args: Args,
    params: ParticleParams,
    workers: Arc<Workers>,
    scopes: PeerScopes,
    spell_storage: SpellStorage,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;

    if !scopes.is_management(params.init_peer_id) && !scopes.is_host(params.init_peer_id) {
        return Err(JError::new(format!(
            "Only management or host peer can deactivate deal"
        )));
    }

    let worker_id = workers.get_worker_id(deal_id)?;

    if !workers.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been deactivated"));
    }

    let spells = spell_storage.get_registered_spells_by(PeerScope::WorkerId(worker_id));

    let host_peer_id = scopes.get_host_peer_id();
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
                CallParams::local(
                    spell_id.clone(),
                    PeerScope::WorkerId(worker_id),
                    host_peer_id,
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

    workers.deactivate_worker(worker_id).await?;

    Ok(())
}

pub(crate) async fn activate_deal(
    args: Args,
    params: ParticleParams,
    workers: Arc<Workers>,
    scopes: PeerScopes,
    services: ParticleAppServices,
    spell_event_bus_api: SpellEventBusApi,
    spell_service_api: SpellServiceApi,
    worker_period_sec: u32,
) -> Result<(), JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;

    if !scopes.is_management(params.init_peer_id) && !scopes.is_host(params.init_peer_id) {
        return Err(JError::new(format!(
            "Only management or host peer can activate deal"
        )));
    }

    let worker_id = workers.get_worker_id(deal_id)?;
    let host_peer_id = scopes.get_host_peer_id();

    if workers.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been activated"));
    }

    let installation_spell_id = services.resolve_alias(
        PeerScope::WorkerId(worker_id),
        "worker-spell".to_string(),
        &params.id,
    )?;

    // same as in decider-distro
    let mut worker_config = TriggerConfig::default();
    worker_config.clock.start_sec = 1;
    worker_config.clock.period_sec = worker_period_sec;

    spell_service_api.set_trigger_config(
        CallParams::local(
            installation_spell_id.clone(),
            PeerScope::WorkerId(worker_id),
            host_peer_id,
            Duration::from_millis(params.ttl as u64),
        ),
        worker_config.clone(),
    )?;

    let trigger_config = from_user_config(&worker_config)?.ok_or(JError::new(format!(
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

    workers.activate_worker(worker_id).await?;
    Ok(())
}

pub(crate) fn is_deal_active(args: Args, workers: Arc<Workers>) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;
    let worker_id = workers.get_worker_id(deal_id)?;
    Ok(JValue::Bool(workers.is_worker_active(worker_id)))
}
