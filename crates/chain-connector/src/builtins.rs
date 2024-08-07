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
use crate::types::{SubnetResolveResult, TxReceiptResult, TxStatus, Worker};
use crate::{ChainConnector, HttpChainConnector};
use ccp_shared::types::CUID;
use futures::FutureExt;
use particle_args::{Args, JError};
use particle_builtins::{wrap, CustomService};
use particle_execution::{ParticleParams, ServiceFunction};
use serde_json::json;
use serde_json::Value as JValue;
use std::collections::HashMap;
use std::sync::Arc;
use types::peer_scope::WorkerId;
use types::DealId;

pub(crate) fn make_connector_builtins(
    connector: Arc<HttpChainConnector>,
) -> HashMap<String, CustomService> {
    let mut builtins = HashMap::new();
    builtins.insert(
        "connector".to_string(),
        CustomService::new(
            vec![
                ("get_deals", make_get_deals_closure(connector.clone())),
                (
                    "register_worker",
                    make_register_worker_closure(connector.clone()),
                ),
                (
                    "get_tx_receipts",
                    make_get_tx_receipts_closure(connector.clone()),
                ),
            ],
            None,
        ),
    );
    // Legacy service name; Can be deprecated and moved to connector in the future
    builtins.insert(
        "subnet".to_string(),
        CustomService::new(
            vec![("resolve", make_resolve_subnet_closure(connector.clone()))],
            None,
        ),
    );
    builtins
}

fn make_resolve_subnet_closure(connector: Arc<HttpChainConnector>) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |args, _params| {
        let connector = connector.clone();
        async move { wrap(resolve_subnet_builtin(connector, args).await) }.boxed()
    }))
}

fn make_get_deals_closure(connector: Arc<HttpChainConnector>) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |_, params| {
        let connector = connector.clone();
        async move { wrap(get_deals_builtin(connector, params).await) }.boxed()
    }))
}

fn make_register_worker_closure(connector: Arc<HttpChainConnector>) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |args, params| {
        let connector = connector.clone();
        async move { wrap(register_worker_builtin(connector, args, params).await) }.boxed()
    }))
}

fn make_get_tx_receipts_closure(connector: Arc<HttpChainConnector>) -> ServiceFunction {
    ServiceFunction::Immut(Box::new(move |args, params| {
        let connector = connector.clone();
        async move { wrap(get_tx_receipts_builtin(connector, args, params).await) }.boxed()
    }))
}

async fn get_deals_builtin(
    connector: Arc<HttpChainConnector>,
    params: ParticleParams,
) -> Result<JValue, JError> {
    if params.init_peer_id != connector.host_id {
        return Err(JError::new(
            "Only the root worker can call connector.get_deals",
        ));
    }

    let deals = connector
        .get_deals()
        .await
        .map_err(|err| JError::new(format!("Failed to get deals: {err}")))?;
    Ok(json!(deals))
}

async fn register_worker_builtin(
    connector: Arc<HttpChainConnector>,
    args: Args,
    params: ParticleParams,
) -> Result<JValue, JError> {
    if params.init_peer_id != connector.host_id {
        return Err(JError::new(
            "Only the root worker can call connector.register_worker",
        ));
    }

    let mut args = args.function_args.into_iter();
    let deal_id: DealId = Args::next("deal_id", &mut args)?;
    let worker_id: WorkerId = Args::next("worker_id", &mut args)?;
    let cu_ids: Vec<CUID> = Args::next("cu_id", &mut args)?;

    if cu_ids.len() != 1 {
        return Err(JError::new("Only one cu_id is allowed"));
    }

    let tx_hash = connector
        .register_worker(&deal_id, worker_id, cu_ids[0])
        .await
        .map_err(|err| JError::new(format!("Failed to register worker: {err}")))?;
    Ok(json!(tx_hash))
}

async fn get_tx_receipts_builtin(
    connector: Arc<HttpChainConnector>,
    args: Args,
    params: ParticleParams,
) -> Result<JValue, JError> {
    if params.init_peer_id != connector.host_id {
        return Err(JError::new(
            "Only the root worker can call connector.get_tx_receipt",
        ));
    }

    let mut args = args.function_args.into_iter();

    let tx_hashes: Vec<String> = Args::next("tx_hashes", &mut args)?;

    let receipts = connector
        .get_tx_receipts(tx_hashes)
        .await
        .map_err(|err| JError::new(format!("Failed to get tx receipts: {err}")))?
        .into_iter()
        .map(|tx_receipt| match tx_receipt {
            Ok(receipt) => match receipt {
                TxStatus::Pending => TxReceiptResult::pending(),
                TxStatus::Processed(receipt) => TxReceiptResult::processed(receipt),
            },
            Err(err) => TxReceiptResult::error(err.to_string()),
        })
        .collect::<Vec<_>>();

    Ok(json!(receipts))
}

async fn resolve_subnet_builtin(
    connector: Arc<HttpChainConnector>,
    args: Args,
) -> Result<JValue, JError> {
    let deal_id: String = Args::next("deal_id", &mut args.function_args.into_iter())?;
    let deal_id = DealId::from(deal_id);

    let workers: eyre::Result<Vec<Worker>> = try {
        if !deal_id.is_valid() {
            Err(eyre::eyre!(
                "Invalid deal id '{}': invalid length",
                deal_id.as_str()
            ))?;
        }

        let units = connector.get_deal_compute_units(&deal_id).await?;
        let workers: Result<Vec<Worker>, _> = units
            .into_iter()
            .map(|unit| Worker::try_from(unit))
            .collect();
        workers?
    };

    let result = match workers {
        Ok(workers) => SubnetResolveResult {
            success: true,
            workers,
            error: vec![],
        },
        Err(err) => SubnetResolveResult {
            success: false,
            workers: vec![],
            error: vec![format!("{}", err)],
        },
    };

    Ok(json!(result))
}
