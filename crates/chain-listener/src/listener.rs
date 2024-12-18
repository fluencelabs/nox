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

use alloy_primitives::{Address, FixedBytes, Uint, U256};
use alloy_sol_types::SolEvent;
use backoff::Error::Permanent;
use std::cmp::min;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::{Debug, Formatter};
use std::future::{pending, Future};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use backoff::future::retry;
use backoff::ExponentialBackoff;
use ccp_rpc_client::CCPRpcHttpClient;
use ccp_shared::proof::BatchRequest;
use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash};
use cpu_utils::PhysicalCoreId;
use hex_utils::encode_hex_0x;

use eyre::{eyre, Report};
use jsonrpsee::core::client::{Client as WsClient, Subscription, SubscriptionClientT};
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::core::{client, JsonValue};
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::{PingConfig, WsClientBuilder};
use libp2p_identity::PeerId;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use tracing::Instrument;

use chain_connector::Offer::ComputeUnit;
use chain_connector::{
    is_commitment_not_active, is_too_many_proofs, CCStatus, ChainConnector, CommitmentId,
    ConnectorError, Deal, OnChainWorkerID, PEER_NOT_EXISTS,
};
use chain_data::{parse_log, peer_id_to_hex, BlockHeader, Log};
use core_distributor::errors::AcquireError;
use core_distributor::types::{AcquireRequest, Assignment, WorkType};
use core_distributor::{CoreDistributor, CUID};
use peer_metrics::ChainListenerMetrics;
use server_config::{ChainConfig, ChainListenerConfig};
use types::DealId;

use crate::event::CommitmentActivated;
use crate::event::{ComputeUnitsMatched, UnitActivated, UnitDeactivated};
use crate::proof_tracker::ProofTracker;
use crate::types::{CUGroups, PhysicalCoreGroups};

const PROOF_POLL_LIMIT: usize = 50;

#[derive(Clone)]
struct OnChainWorker {
    id: OnChainWorkerID,
    cu_ids: Vec<CUID>,
}

impl Debug for OnChainWorker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(chain_worker_id = {}, cu_ids = {:?})",
            self.id,
            self.cu_ids
                .iter()
                .map(|cu| cu.to_string())
                .collect::<Vec<_>>()
        )
    }
}

pub struct ChainListener {
    config: ChainConfig,
    listener_config: ChainListenerConfig,

    chain_connector: Arc<dyn ChainConnector>,
    // To subscribe to chain events
    ws_client: WsClient,

    ccp_client: Option<CCPRpcHttpClient>,

    core_distributor: Arc<dyn CoreDistributor>,

    host_id: PeerId,

    // These settings aren't changed
    // We refresh them on some errors, but it's enough to get them only on start without refreshing
    difficulty: Difficulty,
    // The time when the first epoch starts (aka the contract was deployed)
    init_timestamp: U256,
    epoch_duration: U256,
    min_proofs_per_epoch: U256,
    max_proofs_per_epoch: U256,
    // These settings are changed each epoch
    global_nonce: GlobalNonce,
    current_epoch: U256,
    last_observed_block_timestamp: U256,

    current_commitment: Option<CommitmentId>,

    // the compute units that are in the commitment and not in deals
    cc_compute_units: BTreeMap<CUID, ComputeUnit>,
    // the compute units that are in deals and not in commitment
    active_deals: BTreeMap<DealId, OnChainWorker>,

    proof_tracker: ProofTracker,
    pending_proof_txs: Vec<(String, Vec<CUID>)>,

    // TODO: move out to a separate struct, get rid of Option
    // Subscriptions that are polled when we have commitment
    unit_activated: Option<Subscription<JsonValue>>,
    unit_deactivated: Option<Subscription<JsonValue>>,
    // Subscriptions that are polled always
    heads: Option<Subscription<JsonValue>>,
    commitment_activated: Option<Subscription<JsonValue>>,
    units_matched: Option<Subscription<JsonValue>>,

    metrics: Option<ChainListenerMetrics>,
}

async fn poll_subscription<T>(
    s: &mut Option<Subscription<T>>,
) -> Option<Result<T, serde_json::Error>>
where
    T: DeserializeOwned + Send,
{
    match s {
        Some(ref mut s) => s.next().await,
        None => pending().await,
    }
}

impl ChainListener {
    pub fn new(
        chain_config: ChainConfig,
        ws_client: WsClient,
        listener_config: ChainListenerConfig,
        host_id: PeerId,
        chain_connector: Arc<dyn ChainConnector>,
        core_distributor: Arc<dyn CoreDistributor>,
        ccp_client: Option<CCPRpcHttpClient>,
        persisted_proof_id_dir: PathBuf,
        metrics: Option<ChainListenerMetrics>,
    ) -> Self {
        if ccp_client.is_none() {
            tracing::warn!(target: "chain-listener", "CCP client is not set, will submit mocked proofs");
        }

        Self {
            chain_connector,
            ws_client,
            listener_config,
            config: chain_config,
            host_id,
            difficulty: Difficulty::default(),
            init_timestamp: U256::ZERO,
            global_nonce: GlobalNonce::new([0; 32]),
            current_epoch: U256::ZERO,
            epoch_duration: U256::ZERO,
            min_proofs_per_epoch: U256::ZERO,
            max_proofs_per_epoch: U256::ZERO,
            current_commitment: None,
            cc_compute_units: BTreeMap::new(),
            core_distributor,
            ccp_client,
            pending_proof_txs: vec![],
            unit_activated: None,
            unit_deactivated: None,
            heads: None,
            commitment_activated: None,
            units_matched: None,
            active_deals: BTreeMap::new(),
            metrics,
            proof_tracker: ProofTracker::new(persisted_proof_id_dir),
            last_observed_block_timestamp: U256::ZERO,
        }
    }

    async fn handle_subscription_error(&mut self, event: &str, err: Report) {
        tracing::warn!(target: "chain-listener", "{event} event processing error: {err}");

        let result: eyre::Result<()> = try {
            self.refresh_state().await?;
            self.refresh_subscriptions().await?;
        };

        if let Err(err) = result {
            tracing::error!(target: "chain-listener", "Failed to resubscribe: {err}; Stopping...");
            exit(1);
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let result = tokio::task::Builder::new()
            .name("ChainListener")
            .spawn(async move {
                if let Err(err) = self.set_utility_core().await {
                    tracing::error!(target: "chain-listener", "Failed to set utility core: {err}; Stopping...");
                    exit(1);
                }

                tracing::info!(target: "chain-listener", "Subscribing to chain events");
                if let Err(err) = self.refresh_subscriptions().await {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to chain events: {err}; Stopping...");
                    exit(1);
                }
                tracing::info!(target: "chain-listener", "Subscribed successfully");

                // Proof id should be loaded once on start, there is no reason to update it on refresh

                if let Err(err) = self.proof_tracker.load_state().await {
                    tracing::error!(target: "chain-listener", "Failed to load persisted proof tracker state: {err}; Stopping...");
                    exit(1);
                }

                if let Err(err) = self.refresh_state().await {
                    tracing::error!(target: "chain-listener", "Failed to refresh state: {err}; Stopping...");
                    exit(1);
                }

                tracing::info!(target: "chain-listener", "State successfully refreshed, starting main loop");
                let mut timer = IntervalStream::new(interval(self.listener_config.proof_poll_period));

                loop {
                    tokio::select! {
                        event = poll_subscription(&mut self.heads) => {
                            if let Err(err) = self.process_new_header(event).await {
                                self.handle_subscription_error("newHeads", err).await;
                            }
                        },
                        event = poll_subscription(&mut self.commitment_activated) => {
                            if let Err(err) = self.process_commitment_activated(event).await {
                                self.handle_subscription_error("CommitmentActivated", err).await;
                            }
                        },
                        event = poll_subscription(&mut self.unit_activated) => {
                            if self.unit_activated.is_some() {
                                if let Err(err) = self.process_unit_activated(event).await {
                                    self.handle_subscription_error("UnitActivated", err).await;
                                }
                            }
                        },
                        event = poll_subscription(&mut self.unit_deactivated) => {
                            if self.unit_deactivated.is_some() {
                                 if let Err(err) = self.process_unit_deactivated(event).await {
                                    self.handle_subscription_error("UnitDeactivated", err).await;
                                }
                            }
                        },
                        event = poll_subscription(&mut self.units_matched) => {
                            if let Err(err) = self.process_compute_units_matched(event) {
                                self.handle_subscription_error("ComputeUnitsMatched", err).await;
                            }
                        },
                        _ = timer.next() => {
                            if self.ccp_client.is_some() {
                                if let Err(err) = self.poll_proofs().await {
                                    tracing::warn!(target: "chain-listener", "Failed to poll/submit proofs: {err}");
                                }
                            } else if let Err(err) = self.submit_mocked_proofs().await {
                                tracing::warn!(target: "chain-listener", "Failed to submit mocked proofs: {err}");
                            }


                            if let Err(err) = self.poll_deal_statuses().await {
                                tracing::warn!(target: "chain-listener", "Failed to poll deal statuses: {err}");
                            }

                            if let Err(err) = self.poll_pending_proof_txs().await {
                                tracing::warn!(target: "chain-listener", "Failed to poll pending proof txs: {err}");
                            }

                            // NOTE: we need to update global nonce by timer because sometimes it's stale
                            // at the beginning of the epoch. It's caused by inconsistency between eth-rpc and subscription to newHeads
                            if let Err(err) = self.update_global_nonce().await  {
                                tracing::warn!(target: "chain-listener", "Failed to update global nonce: {err}");
                            }
                        }
                    }
                }
            }
                .in_current_span()
            )
            .expect("Could not spawn task");

        result
    }

    async fn refresh_current_commitment_id(&mut self) -> eyre::Result<()> {
        match self.chain_connector.get_current_commitment_id().await {
            Ok(id) => {
                // This is the only place where `current_commitment` is updated, so it should be fine
                // to observe the metrics it here
                if id != self.current_commitment {
                    if let Some(current_commitment) = &self.current_commitment {
                        self.observe(|m| {
                            m.observe_removed_commitment(current_commitment.to_string())
                        });
                    }
                    if let Some(new_commitment) = &id {
                        self.observe(|m| m.observe_new_commitment(new_commitment.to_string()));
                    }
                }
                self.current_commitment = id;
                Ok(())
            }
            Err(err) => match err {
                ConnectorError::RpcCallError { ref data, .. } => {
                    if data.contains(PEER_NOT_EXISTS) {
                        tracing::info!("Peer doesn't exist on chain. Waiting for market offer");
                        Ok(())
                    } else {
                        tracing::error!(target: "chain-listener", "Failed to get current commitment id: {err}");
                        Err(err.into())
                    }
                }
                _ => {
                    tracing::error!(target: "chain-listener", "Failed to get current commitment id: {err}");
                    Err(err.into())
                }
            },
        }
    }

    async fn refresh_commitment_params(&mut self) -> eyre::Result<()> {
        let init_params =
            self.chain_connector
                .get_cc_init_params()
                .await
                .map_err(|err| {
                    tracing::info!(target: "chain-listener", "Error getting Commitment initial params: {err}");
                    err
                })?;

        tracing::info!(target: "chain-listener","Commitment initial params: difficulty {}, global nonce {}, init_timestamp {}, epoch_duration {}, current_epoch {}, min_proofs_per_epoch {}, max_proofs_per_epoch {}",  init_params.difficulty, init_params.global_nonce, init_params.init_timestamp, init_params.epoch_duration, init_params.current_epoch, init_params.min_proofs_per_epoch, init_params.max_proofs_per_epoch);

        self.difficulty = init_params.difficulty;
        self.init_timestamp = init_params.init_timestamp;

        self.epoch_duration = init_params.epoch_duration;
        self.min_proofs_per_epoch = init_params.min_proofs_per_epoch;
        self.max_proofs_per_epoch = init_params.max_proofs_per_epoch;

        self.set_current_epoch(init_params.current_epoch).await;
        self.set_global_nonce(init_params.global_nonce).await;

        self.observe(|m| {
            m.observe_epoch_settings(
                truncate_to_u64(&self.init_timestamp) as i64,
                truncate_to_u64(&self.epoch_duration) as i64,
            );

            m.observe_allowed_proofs_settings(
                truncate_to_u64(&self.max_proofs_per_epoch) as i64,
                truncate_to_u64(&self.min_proofs_per_epoch) as i64,
            )
        });

        Ok(())
    }

    async fn refresh_state(&mut self) -> eyre::Result<()> {
        loop {
            let result: eyre::Result<()> = try {
                // TODO: can do it once, on start
                self.refresh_commitment_params().await?;
                // but others we need to refresh on each error
                self.refresh_compute_units().await?;
                self.refresh_current_commitment_id().await?;

                if let Some(c) = self.current_commitment.clone() {
                    tracing::info!(target: "chain-listener", "Current commitment id: {}", c);
                    tracing::info!(target: "chain-listener", "Subscribing to unit events");

                    if let Err(err) = self.subscribe_unit_events(&c).await {
                        tracing::warn!(target: "chain-listener", "Failed to subscribe to unit events: {err}");
                        self.refresh_subscriptions().await?;
                    }

                    tracing::info!(target: "chain-listener", "Successfully subscribed to unit events");
                } else {
                    tracing::info!(target: "chain-listener", "Compute peer has no commitment");
                    self.reset_commitment().await?
                }

                if let Some(status) = self.get_commitment_status().await? {
                    tracing::info!(target: "chain-listener", "Current commitment status: {status:?}");

                    match status {
                        CCStatus::Active => {
                            self.refresh_commitment().await?;
                        }

                        CCStatus::Inactive | CCStatus::Failed | CCStatus::Removed => {
                            self.reset_commitment().await?;
                        }
                        CCStatus::WaitDelegation | CCStatus::WaitStart => {
                            tracing::info!(target: "chain-listener", "Waiting for commitment to be activated; Stopping current one");
                            self.stop_commitment().await?
                        }
                        _ => {}
                    }
                }
            };

            if let Err(e) = result {
                tracing::warn!(target: "chain-listener", "Failed to refresh state: {e}");
                tracing::info!(target: "chain-listener", "Retrying in 5 seconds");
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                break;
            }
        }

        Ok(())
    }

    // Allocate one CPU core for utility use
    async fn set_utility_core(&mut self) -> eyre::Result<()> {
        if let Some(ccp_client) = self.ccp_client.as_ref() {
            // We will use the first logical core for utility tasks
            let utility_core = self
                .core_distributor
                .get_system_cpu_assignment()
                .logical_core_ids
                .first()
                .cloned()
                .ok_or(eyre::eyre!("No utility core id"))?;
            measured_request(&self.metrics,
                             retry(ExponentialBackoff::default(), || async {
                                 ccp_client
                                     .realloc_utility_cores(vec![utility_core])
                                     .await
                                     .map_err(|err| {
                                         tracing::warn!(target: "chain-listener", "Error reallocating utility core {utility_core} to CCP, error: {err}. Retrying...");
                                         eyre::eyre!("Error reallocating utility core {utility_core} to CCP, error: {err}")
                                     })?;
                                 Ok(())
                             }),
            ).await?;

            tracing::info!("Utility core {utility_core} successfully reallocated");
        }
        Ok(())
    }

    async fn get_commitment_status(&self) -> eyre::Result<Option<CCStatus>> {
        if let Some(commitment_id) = self.current_commitment.clone() {
            let status = self
                .chain_connector
                .get_commitment_status(commitment_id)
                .await?;
            self.observe(|m| m.observe_commiment_status(status as u64));
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    pub async fn create_ws_client(config: &ChainListenerConfig) -> Result<WsClient, client::Error> {
        let ws_client = retry(ExponentialBackoff::default(), || async {
            let client = WsClientBuilder::default()
                .enable_ws_ping(PingConfig::new().ping_interval(config.ws_ping_period))
                .build(config.ws_endpoint.clone())
                .await
                .map_err(|err| {
                    tracing::warn!(
                        target: "chain-listener",
                        "Error connecting to websocket endpoint {}, error: {}; Retrying...",
                        config.ws_endpoint,
                        err
                    );
                    err
                })?;

            Ok(client)
        })
        .await?;

        tracing::info!(
            target: "chain-listener",
            "Successfully connected to websocket endpoint: {}",
            config.ws_endpoint
        );

        Ok(ws_client)
    }

    async fn subscribe_unit_events(
        &mut self,
        commitment_id: &CommitmentId,
    ) -> Result<(), client::Error> {
        self.unit_activated = Some(
            self.subscribe("logs", self.unit_activated_params(commitment_id))
                .await?,
        );
        self.unit_deactivated = Some(
            self.subscribe("logs", self.unit_deactivated_params(commitment_id))
                .await?,
        );

        Ok(())
    }

    async fn refresh_subscriptions(&mut self) -> Result<(), client::Error> {
        if !self.ws_client.is_connected() {
            self.ws_client = ChainListener::create_ws_client(&self.listener_config).await?;
        }

        // loop because subscriptions can fail and require reconnection, we can't proceed without them
        loop {
            let result: Result<(), client::Error> = try {
                self.heads = Some(self.subscribe("newHeads", rpc_params!["newHeads"]).await?);
                self.commitment_activated =
                    Some(self.subscribe("logs", self.cc_activated_params()).await?);
                self.units_matched =
                    Some(self.subscribe("logs", self.unit_matched_params()).await?);
                if let Some(commitment_id) = self.current_commitment.clone() {
                    self.subscribe_unit_events(&commitment_id).await?;
                }
            };

            match result {
                Ok(_) => {
                    tracing::info!(target: "chain-listener", "Subscriptions refreshed successfully");
                    break;
                }
                Err(err) => match err {
                    client::Error::RestartNeeded(_) => {
                        tracing::warn!(target: "chain-listener", "Failed to refresh subscriptions: {err}; Restart client...");
                        self.ws_client =
                            ChainListener::create_ws_client(&self.listener_config).await?;
                    }
                    _ => {
                        tracing::error!(target: "chain-listener", "Failed to refresh subscriptions: {err}; Retrying...");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                },
            }
        }
        Ok(())
    }

    /// Updates active and pending compute units
    async fn refresh_compute_units(&mut self) -> eyre::Result<()> {
        let mut units = self.chain_connector.get_compute_units().await?;

        self.observe(|m| m.observe_cus_total(units.len() as i64));

        let in_deal: Vec<_> = units.extract_if(|cu| !cu.deal.is_zero()).collect();

        self.observe(|m| m.observe_cus_in_deals(in_deal.len() as i64));

        let current_units: Vec<CUID> = units.iter().map(|unit| CUID::new(unit.id.0)).collect();
        self.core_distributor
            .cleanup_cache(current_units.as_slice());

        self.cc_compute_units = units
            .into_iter()
            .map(|unit| (CUID::new(unit.id.0), unit))
            .collect();

        self.active_deals.clear();
        for cu in &in_deal {
            let cu_id = CUID::new(cu.id.0);
            let deal_id = cu.deal.to_string().into();
            let onchain_worker = self.active_deals.entry(deal_id).or_insert(OnChainWorker {
                id: cu.onchainWorkerId,
                cu_ids: vec![],
            });
            onchain_worker.cu_ids.push(cu_id);
        }

        let active = self
            .cc_compute_units
            .values()
            .filter(|unit| unit.startEpoch <= self.current_epoch);

        let pending = self
            .cc_compute_units
            .values()
            .filter(|unit| unit.startEpoch > self.current_epoch);

        tracing::info!(target: "chain-listener",
            "Compute units mapping: in cc {}/[{} pending], in deal {}",
            self.cc_compute_units.len(),
            pending.clone().count(),
            in_deal.len()
        );

        tracing::info!(target: "chain-listener",
            "Active compute units: {:?}",
            active.map(|cu| cu.id.to_string()).collect::<Vec<_>>()
        );
        tracing::info!(target: "chain-listener",
            "Pending compute units: {:?}",
            pending
                .map(|cu| cu.id.to_string())
                .collect::<Vec<_>>()
        );
        tracing::info!(target: "chain-listener",
            "In deal compute units: {:?}",
            self.active_deals.values()
                .collect::<Vec<_>>()
        );

        // NOTE: cores are released after all the logs to simplify debug on failure
        for worker in self.active_deals.values() {
            self.core_distributor.release_worker_cores(&worker.cu_ids);
            self.acquire_core_for_deal(worker.cu_ids.clone())?;
        }

        Ok(())
    }

    async fn subscribe(
        &self,
        method: &str,
        params: ArrayParams,
    ) -> Result<Subscription<JsonValue>, client::Error> {
        let sub = retry(ExponentialBackoff::default(), || async {
            self
                .ws_client
                .subscribe("eth_subscribe", params.clone(), "eth_unsubscribe")
                .await.map_err(|err| {
                if let client::Error::RestartNeeded(_) = err {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to {method}: {err};");
                    Permanent(err)
                } else {
                    tracing::warn!(target: "chain-listener", "Failed to subscribe to {method}: {err}; Retrying...");
                    backoff::Error::transient(err)
                }
            })
        }).await?;

        Ok(sub)
    }

    fn cc_activated_params(&self) -> ArrayParams {
        let topic = CommitmentActivated::SIGNATURE_HASH.to_string();
        let topics = vec![topic, peer_id_to_hex(self.host_id)];
        rpc_params![
            "logs",
            json!({"address": self.config.diamond_contract_address, "topics": topics})
        ]
    }

    fn unit_activated_params(&self, commitment_id: &CommitmentId) -> ArrayParams {
        let topic = UnitActivated::SIGNATURE_HASH.to_string();
        rpc_params![
            "logs",
            json!({"address": self.config.diamond_contract_address, "topics": vec![topic, encode_hex_0x(commitment_id.0)]})
        ]
    }

    fn unit_deactivated_params(&self, commitment_id: &CommitmentId) -> ArrayParams {
        let topic = UnitDeactivated::SIGNATURE_HASH.to_string();
        rpc_params![
            "logs",
            json!({"address": self.config.diamond_contract_address, "topics": vec![topic, encode_hex_0x(commitment_id.0)]})
        ]
    }

    fn unit_matched_params(&self) -> ArrayParams {
        let topics = vec![
            ComputeUnitsMatched::SIGNATURE_HASH.to_string(),
            peer_id_to_hex(self.host_id),
        ];
        rpc_params![
            "logs",
            json!({"address": self.config.diamond_contract_address, "topics": topics})
        ]
    }

    async fn process_new_header(
        &mut self,
        event: Option<Result<Value, serde_json::Error>>,
    ) -> eyre::Result<()> {
        let header = event.ok_or(eyre!("Failed to process newHeads event: got None"))?;

        let header = BlockHeader::from_json(header?)?;
        let block_number = header.number.as_limbs()[0];
        let block_timestamp = header.timestamp;

        self.last_observed_block_timestamp = block_timestamp;
        self.observe(|m| m.observe_new_block(block_number));

        // `epoch_number = 1 + (block_timestamp - init_timestamp) / epoch_duration`
        let epoch_number = U256::from(1)
            + (Uint::from(block_timestamp) - self.init_timestamp) / self.epoch_duration;
        let epoch_changed = epoch_number > self.current_epoch;

        if epoch_changed {
            // TODO: add epoch_number to metrics

            self.set_current_epoch(epoch_number).await;

            let nonce_updated = self
                .set_global_nonce(self.chain_connector.get_global_nonce().await?)
                .await;

            if !nonce_updated {
                tracing::warn!(target: "chain-listener", "Epoch changed but global nonce hasn't changed. Don't worry, it'll catch up soon");
            }

            tracing::info!(target: "chain-listener", "Global nonce: {}", self.global_nonce);

            if let Some(status) = self.get_commitment_status().await? {
                tracing::info!(target: "chain-listener", "Current commitment status: {status:?}");

                match status {
                    CCStatus::Active => {
                        self.refresh_commitment().await?;
                    }
                    CCStatus::Inactive | CCStatus::Failed | CCStatus::Removed => {
                        self.reset_commitment().await?;
                    }
                    _ => {}
                }
            }
        }
        self.observe(|m| m.observe_processed_block(block_number));
        Ok(())
    }

    async fn process_commitment_activated(
        &mut self,
        event: Option<Result<JsonValue, serde_json::Error>>,
    ) -> eyre::Result<()> {
        let event = event.ok_or(eyre!(
            "Failed to process CommitmentActivated event: got None"
        ))??;
        let log = serde_json::from_value::<Log>(event.clone()).map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to parse CommitmentActivated event: {err}, data: {event}");
            err
        })?;

        let cc_event = parse_log::<CommitmentActivated>(log)?;
        let unit_ids = cc_event.unitIds;
        tracing::info!(target: "chain-listener",
            "Received CommitmentActivated event for commitment: {}, startEpoch: {}, unitIds: {:?}",
            cc_event.commitmentId.to_string(),
            cc_event.startEpoch,
            unit_ids
                .iter()
                .map(FixedBytes::to_string)
                .collect::<Vec<_>>()
        );

        let commitment_id = CommitmentId(cc_event.commitmentId.0);
        self.current_commitment = Some(commitment_id.clone());
        if let Err(err) = self.subscribe_unit_events(&commitment_id).await {
            tracing::warn!(target: "chain-listener", "Failed to subscribe to unit events: {err}");
            self.refresh_subscriptions().await?;
        }

        self.cc_compute_units = unit_ids
            .into_iter()
            .map(|id| {
                (
                    CUID::new(id.0),
                    ComputeUnit {
                        id,
                        deal: Address::ZERO,
                        startEpoch: cc_event.startEpoch,
                        onchainWorkerId: FixedBytes::<32>::ZERO,
                    },
                )
            })
            .collect();

        self.refresh_commitment().await?;

        Ok(())
    }

    async fn process_unit_activated(
        &mut self,
        event: Option<Result<JsonValue, serde_json::Error>>,
    ) -> eyre::Result<()> {
        let event = event.ok_or(eyre!("Failed to process UnitActivated event: got None"))??;

        let log = serde_json::from_value::<Log>(event.clone()).map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to parse UnitActivated event: {err}, data: {event}");
            err
        })?;

        let unit_event = parse_log::<UnitActivated>(log)?;
        tracing::info!(target: "chain-listener",
            "Received UnitActivated event for unit: {}, startEpoch: {}",
            unit_event.unitId,
            unit_event.startEpoch
        );

        self.cc_compute_units.insert(
            CUID::new(unit_event.unitId.0),
            ComputeUnit {
                id: unit_event.unitId,
                deal: Address::ZERO,
                startEpoch: unit_event.startEpoch,
                onchainWorkerId: FixedBytes::<32>::ZERO,
            },
        );

        self.refresh_commitment().await?;
        Ok(())
    }

    /// Unit goes to Deal
    async fn process_unit_deactivated(
        &mut self,
        event: Option<Result<JsonValue, serde_json::Error>>,
    ) -> eyre::Result<()> {
        let event = event.ok_or(eyre!("Failed to process UnitDeactivated event: got None"))??;
        let log = serde_json::from_value::<Log>(event.clone()).map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to parse UnitDeactivated event: {err}, data: {event}");
            err
        })?;
        let unit_event = parse_log::<UnitDeactivated>(log)?;
        let unit_id = CUID::new(unit_event.unitId.0);
        tracing::info!(target: "chain-listener",
            "Received UnitDeactivated event for unit: {}",
            unit_event.unitId.to_string()
        );
        self.cc_compute_units.remove(&unit_id);
        self.refresh_commitment().await?;
        self.acquire_core_for_deal(vec![unit_id])?;
        Ok(())
    }

    fn process_compute_units_matched(
        &mut self,
        event: Option<Result<JsonValue, serde_json::Error>>,
    ) -> eyre::Result<()> {
        let event = event.ok_or(eyre!("Failed to process DealMatched event: got None"))??;
        let log = serde_json::from_value::<Log>(event.clone()).map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to parse DealMatched event: {err}, data: {event}");
            err
        })?;
        let deal_event = parse_log::<ComputeUnitsMatched>(log)?;
        tracing::info!(target: "chain-listener",
            "Received DealMatched event for deal: {}",
            deal_event.deal
        );

        let cu_ids: Vec<_> = deal_event
            .cuIds
            .into_iter()
            .map(|cu| CUID::new(cu.0))
            .collect();
        let cu_ids_len = cu_ids.len();
        self.active_deals.insert(
            deal_event.deal.to_string().into(),
            OnChainWorker {
                id: deal_event.onchainWorkerId,
                cu_ids,
            },
        );

        self.observe(|m| m.observe_cus_in_deals_added(cu_ids_len as i64));

        Ok(())
    }

    fn get_cu_groups(&self) -> CUGroups {
        let mut priority_units: Vec<CUID> = Vec::new();
        let mut non_priority_units: Vec<CUID> = Vec::new();
        let mut pending_units: Vec<CUID> = Vec::new();
        let mut finished_units: Vec<CUID> = Vec::new();
        for (cuid, cu) in &self.cc_compute_units {
            if cu.startEpoch <= self.current_epoch {
                let count = self.proof_tracker.get_proof_counter(&cuid);
                if count < self.min_proofs_per_epoch {
                    priority_units.push(*cuid)
                } else if count >= self.max_proofs_per_epoch {
                    finished_units.push(*cuid)
                } else {
                    non_priority_units.push(*cuid)
                }
            } else {
                pending_units.push(*cuid);
            }
        }
        CUGroups {
            priority_units,
            non_priority_units,
            pending_units,
            finished_units,
        }
    }

    /// Send GlobalNonce, Difficulty and Core<>CUID mapping (full commitment info) to CCP
    async fn refresh_commitment(&self) -> eyre::Result<()> {
        if self.cc_compute_units.is_empty() || self.current_commitment.is_none() {
            self.stop_commitment().await?;
            return Ok(());
        }

        if self.active_units_count() == 0 {
            tracing::info!(target: "chain-listener", "No active units found in this epoch {}", self.current_epoch);
            self.stop_commitment().await?;
            return Ok(());
        }

        tracing::info!(target: "chain-listener",
            "Refreshing commitment, active compute units: {}",
            self.cc_compute_units
                .keys()
                .map(CUID::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        tracing::info!(target: "chain-listener", "Global nonce: {}", self.global_nonce);
        tracing::info!(target: "chain-listener", "Difficulty: {}", self.difficulty);

        let ccp_client = match &self.ccp_client {
            Some(ccp_client) => ccp_client,
            None => return Ok(()),
        };

        let cu_groups = self.get_cu_groups();
        tracing::trace!(target: "chain-listener", "cu_groups {:?}", cu_groups);

        let cc_cores = self.acquire_cores_for_cc(&cu_groups)?;
        tracing::trace!(target: "chain-listener", "cc_cores {:?}", cc_cores);

        let mut cu_allocation: HashMap<PhysicalCoreId, CUID> = HashMap::new();

        if cu_groups.all_min_proofs_found() {
            tracing::info!(target: "chain-listener", "All CUs found minimal number of proofs {} in current epoch {}", self.min_proofs_per_epoch, self.current_epoch);
            if cu_groups.all_max_proofs_found() {
                tracing::info!(target: "chain-listener", "All CUs found max number of proofs {} in current epoch {}", self.max_proofs_per_epoch ,self.current_epoch);
                self.stop_commitment().await?;
                return Ok(());
            } else {
                // All CUs were proven, now let's work on submitting proofs for every CU until MAX_PROOF_COUNT is reached
                let non_priority_cores_mapping = cc_cores
                    .non_priority_cores
                    .iter()
                    .cloned()
                    .zip(cu_groups.non_priority_units.iter().cloned());
                cu_allocation.extend(non_priority_cores_mapping);

                // Assign "pending cores" to help generate proofs for "non priority units"
                let mut non_priority_units = cu_groups.non_priority_units.iter().cycle();
                cc_cores.pending_cores.iter().for_each(|core| {
                    if let Some(non_priority_unit) = non_priority_units.next() {
                        cu_allocation.insert(*core, *non_priority_unit);
                    }
                });
            }
        } else {
            // Use assigned cores to calculate proofs for CUs who haven't reached MIN_PROOF_COUNT yet
            let priority_cores_mapping = cc_cores
                .priority_cores
                .iter()
                .zip(cu_groups.priority_units.iter());
            cu_allocation.extend(priority_cores_mapping);

            // Use all spare cores to help CUs to reach MIN_PROOF_COUNT
            let spare_cores: BTreeSet<_> = cc_cores
                .non_priority_cores
                .into_iter()
                .chain(cc_cores.pending_cores.into_iter())
                .chain(cc_cores.finished_cores.into_iter())
                .collect();

            let mut units = cu_groups.priority_units.iter().cycle();
            spare_cores.iter().for_each(|core| {
                if let Some(unit) = units.next() {
                    cu_allocation.insert(*core, *unit);
                }
            });
        }

        tracing::info!(target: "chain-listener",
            "Sending commitment to CCP: global_nonce: {}, difficulty: {}, cores: {:?}",
            self.global_nonce,
            self.difficulty,
            cu_allocation.iter().map(|(core, unit)| format!("{}: {}", core, unit))
            .collect::<Vec<_>>()
        );

        measured_request(
            &self.metrics,
            ccp_client.on_active_commitment(self.global_nonce, self.difficulty, cu_allocation),
        )
        .await
        .map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to send commitment to CCP: {err}");
            eyre::eyre!("Failed to send commitment to CCP: {err}")
        })?;

        Ok(())
    }

    fn acquire_cores_for_cc(&self, cu_groups: &CUGroups) -> eyre::Result<PhysicalCoreGroups> {
        let mut units = vec![];
        units.extend(&cu_groups.priority_units);
        units.extend(&cu_groups.non_priority_units);
        units.extend(&cu_groups.pending_units);
        units.extend(&cu_groups.finished_units);

        // Release all ccp units to allow the core distributor to assign them again
        // without that action availability count will be wrong
        self.core_distributor.release_worker_cores(&units);

        let cores = self
            .core_distributor
            .acquire_worker_cores(AcquireRequest::new(
                units.to_vec(),
                WorkType::CapacityCommitment,
            ));

        fn filter(units: &[CUID], assignment: &Assignment) -> Vec<PhysicalCoreId> {
            units
                .iter()
                .filter_map(|cuid| {
                    assignment
                        .cuid_cores
                        .get(cuid)
                        .map(|data| data.physical_core_id)
                })
                .collect()
        }

        match cores {
            Ok(assignment) => {
                let priority_units = filter(&cu_groups.priority_units, &assignment);
                let non_priority_units = filter(&cu_groups.non_priority_units, &assignment);
                let pending_units = filter(&cu_groups.pending_units, &assignment);
                let finished_units = filter(&cu_groups.finished_units, &assignment);

                Ok(PhysicalCoreGroups {
                    priority_cores: priority_units,
                    non_priority_cores: non_priority_units,
                    pending_cores: pending_units,
                    finished_cores: finished_units,
                })
            }
            Err(AcquireError::NotFoundAvailableCores {
                required,
                available,
                ..
            }) => {
                tracing::warn!(target: "chain-listener", "Found {required} CUs in the Capacity Commitment, but Nox has only {available} Cores available for CC");
                let assign_units = units.iter().take(available).cloned().collect();
                let assignment =
                    self.core_distributor
                        .acquire_worker_cores(AcquireRequest::new(
                            assign_units,
                            WorkType::CapacityCommitment,
                        ))?;
                let priority_cores = filter(&cu_groups.priority_units, &assignment);
                let non_priority_cores = filter(&cu_groups.non_priority_units, &assignment);
                let pending_cores = filter(&cu_groups.pending_units, &assignment);
                let finished_cores = filter(&cu_groups.finished_units, &assignment);

                Ok(PhysicalCoreGroups {
                    priority_cores,
                    non_priority_cores,
                    pending_cores,
                    finished_cores,
                })
            }
        }
    }

    fn acquire_core_for_deal(&self, cu_ids: Vec<CUID>) -> eyre::Result<()> {
        self.core_distributor
            .acquire_worker_cores(AcquireRequest::new(cu_ids, WorkType::Deal))?;
        Ok(())
    }

    /// Should be called only if Commitment is Inactive, Failed, Removed or not exists
    async fn reset_commitment(&mut self) -> eyre::Result<()> {
        self.cc_compute_units.clear();
        self.active_deals.clear();
        self.current_commitment = None;
        self.stop_commitment().await?;

        self.observe(|m| m.observe_cus_in_deals(0));

        Ok(())
    }

    async fn stop_commitment(&self) -> eyre::Result<()> {
        tracing::info!(target: "chain-listener", "Stopping current commitment");
        if let Some(ref ccp_client) = self.ccp_client {
            measured_request(&self.metrics,
                             ccp_client.on_no_active_commitment(),
            ).await.map_err(|err| {
                tracing::error!(target: "chain-listener", "Failed to send no active commitment to CCP: {err}");
                eyre::eyre!("Failed to send no active commitment to CCP: {err}")
            })?;
        }
        Ok(())
    }

    /// Submit Mocked Proofs for all active compute units.
    /// Mocked Proof has result_hash == difficulty and random local_nonce
    async fn submit_mocked_proofs(&mut self) -> eyre::Result<()> {
        if self.current_commitment.is_none() {
            return Ok(());
        }

        let all_compute_units = self.cc_compute_units.keys().cloned().collect::<Vec<_>>();
        let mut units = vec![];
        let mut local_nonces = vec![];
        let mut result_hashes = vec![];
        for cuid in all_compute_units.into_iter() {
            let proof_sent = self.proof_tracker.get_proof_counter(&cuid);
            if proof_sent < self.max_proofs_per_epoch {
                units.push(cuid);
                local_nonces.push(LocalNonce::random());
                result_hashes.push(ResultHash::from_slice(*self.difficulty.as_ref()));
            }
        }

        self.submit_proofs(units, local_nonces, result_hashes)
            .await?;
        Ok(())
    }

    async fn update_global_nonce(&mut self) -> eyre::Result<()> {
        let nonce = self.chain_connector.get_global_nonce().await?;
        let nonce_changed = self.set_global_nonce(nonce).await;
        if nonce_changed {
            self.refresh_commitment().await?;
        }
        Ok(())
    }

    fn is_epoch_ending(&self) -> bool {
        let window = Uint::from(self.listener_config.epoch_end_window.as_secs());
        let next_epoch_start = self.init_timestamp + self.epoch_duration * self.current_epoch;
        tracing::debug!(target: "chain-listener", "Next epoch start: {}, last observed block timestamp: {}", next_epoch_start, self.last_observed_block_timestamp);
        next_epoch_start
            .checked_sub(self.last_observed_block_timestamp)
            .unwrap_or(Uint::ZERO)
            < window
    }

    async fn poll_proofs(&mut self) -> eyre::Result<()> {
        if self.current_commitment.is_none() || self.cc_compute_units.is_empty() {
            return Ok(());
        }

        if let Some(ref ccp_client) = self.ccp_client {
            let batch_requests = self.get_batch_request();

            if batch_requests.is_empty() {
                tracing::debug!(target: "chain-listener", "No compute units to poll proofs for. Probably all CUs have reached max proofs count");
                return Ok(());
            }

            let proof_batches = if self.is_epoch_ending() {
                let last_known_proofs = batch_requests
                    .into_iter()
                    .map(|(cu_id, req)| (cu_id, req.last_seen_proof_idx))
                    .collect();

                tracing::debug!(target: "chain-listener", "Polling proofs after {:?}", last_known_proofs);
                measured_request(
                    &self.metrics,
                    ccp_client.get_proofs_after(last_known_proofs, PROOF_POLL_LIMIT),
                )
                .await
                .map_err(|err| eyre::eyre!("Failed to poll proofs from ccp: {err}"))?
            } else {
                tracing::debug!(target: "chain-listener", "Polling proofs after {:?}, min batch count: {}, max batch count: {}", batch_requests, self.listener_config.min_batch_count, self.listener_config.max_batch_count);
                measured_request(
                    &self.metrics,
                    ccp_client.get_batch_proofs_after(
                        batch_requests,
                        self.listener_config.min_batch_count,
                        self.listener_config.max_batch_count,
                    ),
                )
                .await
                .map_err(|err| eyre::eyre!("Failed to poll batched proofs from ccp: {err}"))?
            };

            if !proof_batches.is_empty() {
                let mut unit_ids = Vec::new();
                let mut local_nonces = Vec::new();
                let mut result_hashes = Vec::new();
                let batch_count = proof_batches.len();
                let mut skipped_proofs_count = 0;
                for batch in proof_batches.into_iter() {
                    for proof in batch.proof_batches.into_iter() {
                        if proof.id.global_nonce != self.global_nonce
                            || proof.id.difficulty != self.difficulty
                        {
                            tracing::debug!(target: "chain-listener", "Proof (id={}, global nonce={}, difficulty={}) doesn't match current nonce {} and/or difficulty {}. Skipping..", proof.id.idx, proof.id.global_nonce, proof.id.difficulty, self.global_nonce, self.difficulty);
                            skipped_proofs_count += 1;
                            continue;
                        }

                        unit_ids.push(proof.cu_id);
                        local_nonces.push(proof.local_nonce);
                        result_hashes.push(proof.result_hash);
                        self.proof_tracker
                            .observe_proof(proof.cu_id, proof.id.idx)
                            .await;
                    }
                }

                tracing::info!(target: "chain-listener", "Found {} proofs in {} batches from polling, skipped {} proofs", result_hashes.len(), batch_count, skipped_proofs_count);

                if !result_hashes.is_empty() {
                    self.submit_proofs(unit_ids, local_nonces, result_hashes)
                        .await?;
                }
            } else {
                tracing::debug!(target: "chain-listener", "No proofs found from polling");
            }
        }

        Ok(())
    }

    async fn submit_proofs(
        &mut self,
        unit_ids: Vec<CUID>,
        local_nonces: Vec<LocalNonce>,
        result_hashes: Vec<ResultHash>,
    ) -> eyre::Result<()> {
        let submit = retry(ExponentialBackoff::default(), || async {
            self.chain_connector.submit_proofs(unit_ids.clone(), local_nonces.clone(), result_hashes.clone()).await.map_err(|err| {
                match err {
                    ConnectorError::RpcCallError { .. } => { Permanent(err) }
                    _ => {
                        tracing::warn!(target: "chain-listener", "Failed to submit proof: {err}. Retrying..");
                        backoff::Error::transient(err)
                    }
                }
            })
        })
            .await;

        match submit {
            Err(err) => {
                match err {
                    ConnectorError::RpcCallError { ref data, .. } => {
                        // TODO: track proofs count per epoch and stop at maxProofsPerEpoch
                        if is_too_many_proofs(data) {
                            tracing::info!(target: "chain-listener", "Too many proofs found for some compute unit" );

                            // NOTE: it should be removed from contracts

                            Ok(())
                        } else if is_commitment_not_active(data) {
                            tracing::info!(target: "chain-listener", "Submit proof returned commitment is not active error");
                            let status = self.get_commitment_status().await?;
                            if let Some(status) = status {
                                tracing::info!(target: "chain-listener", "Current commitment status: {status:?}");
                            }

                            self.reset_commitment().await?;
                            Ok(())
                        } else {
                            // TODO: catch more contract asserts like "Proof is not valid" and "Proof is bigger than difficulty"
                            tracing::error!(target: "chain-listener", "Failed to submit proofs {}", err.to_string().replace("\n", " "));
                            tracing::error!(target: "chain-listener", "Units {:?} nonces {:?} result hashes {:?}", unit_ids, local_nonces, result_hashes);
                            // In case of contract errors we just skip these proofs and continue
                            Ok(())
                        }
                    }
                    _ => {
                        tracing::error!(target: "chain-listener", "Failed to submit proof: {err}");
                        tracing::error!(target: "chain-listener", "Units {:?} nonces {:?} result hashes {:?}", unit_ids, local_nonces, result_hashes);
                        self.observe(|m| m.observe_proof_failed());
                        Err(err.into())
                    }
                }
            }
            Ok(tx_id) => {
                tracing::info!(target: "chain-listener", "Successfully submitted {} proofs, txHash: {tx_id}", unit_ids.len());
                self.observe(|m| m.observe_proofs_submitted(unit_ids.len() as u64));
                self.pending_proof_txs.push((tx_id, unit_ids));

                Ok(())
            }
        }
    }

    async fn poll_deal_statuses(&mut self) -> eyre::Result<()> {
        if self.active_deals.is_empty() {
            return Ok(());
        }

        let statuses = retry(ExponentialBackoff::default(), || async {
            let s = self.chain_connector.get_deal_statuses(self.active_deals.keys().cloned().collect()).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to poll deal statuses: {err}; Retrying...");
                eyre!("Failed to poll deal statuses: {err}; Retrying...")
            })?;

            Ok(s)
        })
            .await?;

        for (status, (deal_id, worker)) in statuses
            .into_iter()
            .zip(self.active_deals.clone().into_iter())
        {
            match status {
                Ok(status) => match status {
                    Deal::Status::INSUFFICIENT_FUNDS | Deal::Status::ENDED => {
                        tracing::info!(target: "chain-listener", "Deal {deal_id} status: {status:?}; Exiting...");
                        self.exit_deal(&deal_id, worker.id).await?;
                        tracing::info!(target: "chain-listener", "Exited deal {deal_id} successfully");
                    }
                    _ => {}
                },
                Err(err) => {
                    tracing::error!(target: "chain-listener", "Failed to get deal status for {deal_id}: {err}");
                }
            }
        }

        Ok(())
    }

    async fn exit_deal(
        &mut self,
        deal_id: &DealId,
        onchain_worker_id: OnChainWorkerID,
    ) -> eyre::Result<()> {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(3)),
            ..ExponentialBackoff::default()
        };

        retry(backoff, || async {
            self.chain_connector.exit_deal(deal_id, onchain_worker_id).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to exit deal {deal_id}: {err}");
                eyre!("Failed to exit deal {deal_id}: {err}; Retrying...")
            })?;
            Ok(())
        })
        .await?;

        let removed_deal = self.active_deals.remove(deal_id);

        if let Some(removed_deal) = removed_deal {
            self.observe(|m| m.observe_cus_in_deals_removed(removed_deal.cu_ids.len() as i64));
        }

        Ok(())
    }

    async fn poll_pending_proof_txs(&mut self) -> eyre::Result<()> {
        if self.pending_proof_txs.is_empty() {
            return Ok(());
        }

        let statuses = retry(ExponentialBackoff::default(), || async {
            let txs = self.pending_proof_txs.iter().map(|(tx, _)| tx).cloned().collect();
            let s = self.chain_connector.get_tx_statuses(txs).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to poll pending proof txs statuses: {err}");
                eyre!("Failed to poll pending proof txs statuses: {err}; Retrying...")
            })?;

            Ok(s)
        })
            .await?;

        let mut refresh_neeeded = false;
        let mut stats_updated = false;
        for (status, (tx_hash, cu_ids)) in statuses
            .into_iter()
            .zip(self.pending_proof_txs.clone().into_iter())
        {
            match status {
                Ok(Some(status)) => {
                    if status {
                        tracing::info!(target: "chain-listener", "Proof tx {tx_hash} confirmed");
                        stats_updated = true;
                        for cu_id in cu_ids {
                            let counter = self.proof_tracker.confirm_proof(cu_id).await;

                            if counter == self.max_proofs_per_epoch {
                                tracing::info!(target: "chain-listener", "Compute unit {cu_id} submitted maximum proofs in the current epoch {}", self.current_epoch);
                                // need to call refresh commitment to make some cores to help others
                                refresh_neeeded = true;
                            } else {
                                if counter >= self.min_proofs_per_epoch {
                                    tracing::info!(target: "chain-listener", "Compute unit {cu_id} submitted minimum proofs in the current epoch {}", self.current_epoch);
                                    // need to call refresh commitment to make some cores to help others
                                    refresh_neeeded = true;
                                }
                            }
                        }
                        self.observe(|m| m.observe_proof_tx_success());
                    } else {
                        tracing::warn!(target: "chain-listener", "Proof tx {tx_hash} not confirmed");
                        self.observe(|m| m.observe_proof_tx_failed(tx_hash.to_string()));
                    }

                    self.pending_proof_txs.retain(|(tx, _)| tx != &tx_hash);
                }
                Ok(None) => {
                    tracing::debug!(target: "chain-listener", "Proof tx {tx_hash} is pending");
                }
                Err(err) => {
                    tracing::debug!(target: "chain-listener", "Failed to get tx receipt for {tx_hash}: {err}");
                }
            }
        }

        if refresh_neeeded {
            self.refresh_commitment().await?;
        }

        if stats_updated {
            tracing::debug!(target: "chain-listener", "Confirmed proofs count: {:?}", self.proof_tracker.get_proof_counters().iter().map(|(cu, count)| format!("{}: {}", cu, count)).collect::<Vec<_>>());
        }

        Ok(())
    }

    async fn set_current_epoch(&mut self, epoch_number: U256) {
        self.observe(|m| m.observe_current_epoch(truncate_to_u64(&epoch_number) as i64));

        self.current_epoch = epoch_number;
        self.proof_tracker.set_current_epoch(epoch_number).await;
    }

    /// Returns true if global nonce was updated
    async fn set_global_nonce(&mut self, global_nonce: GlobalNonce) -> bool {
        self.global_nonce = global_nonce;
        self.proof_tracker.set_global_nonce(global_nonce).await
    }

    fn observe<F>(&self, f: F)
    where
        F: FnOnce(&ChainListenerMetrics),
    {
        if let Some(metrics) = self.metrics.as_ref() {
            f(metrics);
        }
    }

    fn active_units_count(&self) -> usize {
        self.cc_compute_units
            .iter()
            .filter(|(_, cu)| cu.startEpoch <= self.current_epoch)
            .count()
    }
    fn get_batch_request(&self) -> HashMap<CUID, BatchRequest> {
        let mut batch_request = HashMap::new();
        for cu_id in self.cc_compute_units.keys() {
            let sent_proofs_count = self.proof_tracker.get_proof_counter(cu_id);
            let proofs_needed = self
                .max_proofs_per_epoch
                .checked_sub(sent_proofs_count)
                .unwrap_or(Uint::ZERO)
                .as_limbs()[0];

            if proofs_needed > 0 {
                let request = BatchRequest {
                    last_seen_proof_idx: self.proof_tracker.get_last_submitted_proof_id(cu_id),
                    proof_batch_size: min(
                        proofs_needed as usize,
                        self.listener_config.max_proof_batch_size,
                    ),
                };

                batch_request.insert(*cu_id, request);
            }
        }
        batch_request
    }
}

// measure the request execution time and store it in the metrics
async fn measured_request<Fut, R, E>(
    metrics: &Option<ChainListenerMetrics>,
    fut: Fut,
) -> Result<R, E>
where
    Fut: Future<Output = Result<R, E>> + Sized,
{
    metrics.as_ref().inspect(|m| m.observe_ccp_request());
    let start = Instant::now();
    let result = fut.await;
    let elapsed = start.elapsed();
    metrics
        .as_ref()
        .inspect(|m| m.observe_ccp_reply(elapsed.as_millis() as f64));
    result
}

fn truncate_to_u64(value: &U256) -> u64 {
    value.as_limbs()[0]
}
