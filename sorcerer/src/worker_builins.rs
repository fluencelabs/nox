/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
use workers::{PeerScopes, WorkerParams, Workers, CUID};

pub(crate) async fn create_worker(
    args: Args,
    params: ParticleParams,
    scopes: PeerScopes,
    workers: Arc<Workers>,
) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;
    let cu_ids: Vec<CUID> = Args::next("cu_ids", &mut args)?;

    if !scopes.is_management(params.init_peer_id) && !scopes.is_host(params.init_peer_id) {
        return Err(JError::new(
            "Only management or host peer can create worker",
        ));
    }

    Ok(JValue::String(
        workers
            .create_worker(WorkerParams::new(
                deal_id.into(),
                params.init_peer_id,
                cu_ids,
            ))
            .await?
            .to_string(),
    ))
}

pub(crate) fn get_worker_peer_id(args: Args, workers: Arc<Workers>) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let deal_id: String = Args::next("deal_id", &mut args)?;
    Ok(JValue::Array(
        workers
            .get_worker_id(deal_id.into())
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
            let is_worker_creator = params.init_peer_id == worker_creator;
            if !is_worker_creator
                && !scopes.is_host(params.init_peer_id)
                && !scopes.is_management(params.init_peer_id)
            {
                return Err(JError::new(format!("Worker {worker_id} can be removed only by worker creator {worker_creator}, host or a host manager")));
            }

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
            workers.remove_worker(worker_id).await?;
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
        return Err(JError::new(
            "Only management or host peer can deactivate deal",
        ));
    }

    let worker_id = workers.get_worker_id(deal_id.into())?;

    if !workers.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been deactivated"));
    }

    let spells = spell_storage.get_registered_spells_by(PeerScope::WorkerId(worker_id));

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
                    PeerScope::WorkerId(worker_id),
                    spell_id.clone(),
                    worker_id.into(),
                    Duration::from_millis(params.ttl as u64),
                ),
                TriggerConfig::default(),
            )
            .await
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
        return Err(JError::new(
            "Only management or host peer can activate deal",
        ));
    }

    let worker_id = workers.get_worker_id(deal_id.into())?;

    if workers.is_worker_active(worker_id) {
        return Err(JError::new("Deal has already been activated"));
    }

    let installation_spell_id = services
        .resolve_alias(
            PeerScope::WorkerId(worker_id),
            "worker-spell".to_string(),
            &params.id,
        )
        .await?;

    // same as in decider-distro
    let mut worker_config = TriggerConfig::default();
    worker_config.clock.start_sec = 1;
    worker_config.clock.period_sec = worker_period_sec;

    spell_service_api
        .set_trigger_config(
            CallParams::local(
                PeerScope::WorkerId(worker_id),
                installation_spell_id.clone(),
                worker_id.into(),
                Duration::from_millis(params.ttl as u64),
            ),
            worker_config.clone(),
        )
        .await?;

    let trigger_config = from_user_config(&worker_config)?.ok_or(JError::new(
        "Deal activation failed due to failure to parse trigger config",
    ))?;

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
    let worker_id = workers.get_worker_id(deal_id.into())?;
    let is_active = workers.is_worker_active(worker_id);
    Ok(JValue::Bool(is_active))
}
