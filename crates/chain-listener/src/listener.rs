use alloy_primitives::{Address, FixedBytes, Uint, U256};
use alloy_sol_types::SolEvent;
use backoff::Error::Permanent;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::pending;
use std::ops::Add;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use backoff::future::retry;
use backoff::ExponentialBackoff;
use ccp_rpc_client::CCPRpcHttpClient;
use ccp_shared::proof::{CCProof, CCProofId, ProofIdx};
use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash};
use cpu_utils::PhysicalCoreId;

use eyre::{eyre, Report};
use jsonrpsee::core::client::{Client as WsClient, Subscription, SubscriptionClientT};
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::core::{client, JsonValue};
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::WsClientBuilder;
use libp2p_identity::PeerId;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use chain_connector::Offer::ComputeUnit;
use chain_connector::{
    is_commitment_not_active, is_too_many_proofs, CCStatus, ChainConnector, CommitmentId,
    ConnectorError, Deal, PEER_NOT_EXISTS,
};
use chain_data::{parse_log, peer_id_to_hex, Log};
use core_manager::errors::AcquireError;
use core_manager::types::{AcquireRequest, Assignment, WorkType};
use core_manager::{CoreManager, CoreManagerFunctions, CUID};
use server_config::{ChainConfig, ChainListenerConfig};
use types::DealId;

use crate::event::cc_activated::CommitmentActivated;
use crate::event::{ComputeUnitMatched, UnitActivated, UnitDeactivated};
use crate::persistence;

const PROOF_POLL_LIMIT: usize = 50;

pub struct ChainListener {
    config: ChainConfig,
    listener_config: ChainListenerConfig,

    chain_connector: Arc<ChainConnector>,
    ws_client: WsClient,
    ccp_client: Option<CCPRpcHttpClient>,
    core_manager: Arc<CoreManager>,

    host_id: PeerId,

    difficulty: Difficulty,
    init_timestamp: U256,
    global_nonce: GlobalNonce,
    current_epoch: U256,
    epoch_duration: U256,
    min_proofs_per_epoch: U256,
    max_proofs_per_epoch: U256,

    proof_counter: BTreeMap<CUID, U256>,
    current_commitment: Option<CommitmentId>,

    cc_compute_units: BTreeMap<CUID, ComputeUnit>,

    active_deals: BTreeMap<DealId, CUID>,

    /// Resets every epoch
    last_submitted_proof_id: ProofIdx,
    pending_proof_txs: Vec<(String, CUID)>,
    persisted_proof_id_dir: PathBuf,

    unit_activated: Option<Subscription<JsonValue>>,
    unit_deactivated: Option<Subscription<JsonValue>>,
    heads: Option<Subscription<JsonValue>>,
    commitment_activated: Option<Subscription<JsonValue>>,
    unit_matched: Option<Subscription<JsonValue>>,
}

async fn poll_subscription<T>(s: &mut Option<Subscription<T>>) -> Option<Result<T, client::Error>>
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
        chain_connector: Arc<ChainConnector>,
        core_manager: Arc<CoreManager>,
        ccp_client: Option<CCPRpcHttpClient>,
        persisted_proof_id_dir: PathBuf,
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
            proof_counter: BTreeMap::new(),
            current_commitment: None,
            cc_compute_units: BTreeMap::new(),
            core_manager,
            ccp_client,
            last_submitted_proof_id: ProofIdx::zero(),
            pending_proof_txs: vec![],
            persisted_proof_id_dir,
            unit_activated: None,
            unit_deactivated: None,
            heads: None,
            commitment_activated: None,
            unit_matched: None,
            active_deals: BTreeMap::new(),
        }
    }

    pub async fn handle_subscription_error(&mut self, event: &str, err: Report) {
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
                        event = poll_subscription(&mut self.unit_matched) => {
                            if let Err(err) = self.process_unit_matched(event) {
                                self.handle_subscription_error("ComputeUnitMatched", err).await;
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
                        }
                    }
                }
            })
            .expect("Could not spawn task");

        result
    }

    async fn refresh_current_commitment_id(&mut self) -> eyre::Result<()> {
        match self.chain_connector.get_current_commitment_id().await {
            Ok(id) => {
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

        self.difficulty = init_params.difficulty;
        self.init_timestamp = init_params.init_timestamp;
        self.global_nonce = init_params.global_nonce;
        self.epoch_duration = init_params.epoch_duration;
        self.current_epoch = init_params.current_epoch;
        self.min_proofs_per_epoch = init_params.min_proofs_per_epoch;
        self.max_proofs_per_epoch = init_params.max_proofs_per_epoch;

        tracing::info!(target: "chain-listener","Commitment initial params: difficulty {}, global nonce {}, init_timestamp {}, epoch_duration {}, current_epoch {}, min_proofs_per_epoch {}, max_proofs_per_epoch {}",  init_params.difficulty, init_params.global_nonce, init_params.init_timestamp, init_params.epoch_duration, init_params.current_epoch, init_params.min_proofs_per_epoch, init_params.max_proofs_per_epoch);
        Ok(())
    }

    async fn refresh_state(&mut self) -> eyre::Result<()> {
        loop {
            let result: eyre::Result<()> = try {
                self.refresh_commitment_params().await?;
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

                self.load_proof_id().await?;
            };

            if let Err(e) = result {
                tracing::warn!(target: "chain-listener", "Failed to refresh compute units: {e}");
                tracing::info!(target: "chain-listener", "Retrying in 5 seconds");
                tokio::time::sleep(Duration::from_secs(5)).await;
            } else {
                break;
            }
        }

        Ok(())
    }

    pub async fn reset_proof_id(&mut self) -> eyre::Result<()> {
        self.set_proof_id(ProofIdx::zero()).await
    }

    pub async fn set_proof_id(&mut self, proof_id: ProofIdx) -> eyre::Result<()> {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(3)),
            ..ExponentialBackoff::default()
        };

        let write = retry(backoff, || async {
            persistence::persist_proof_id(
                &self.persisted_proof_id_dir,
                self.last_submitted_proof_id,
                self.current_epoch,
            ).await.map_err(|err|{
                tracing::warn!(target: "chain-listener", "Failed to persist proof id: {err}; Retrying...");
                eyre!(err)
            })?;
            Ok(())
        }).await;

        if let Err(err) = write {
            tracing::warn!(target: "chain-listener", "Failed to persist proof id: {err}; Ignoring..");
        }

        self.last_submitted_proof_id = proof_id;
        tracing::info!(target: "chain-listener", "Persisted proof id {proof_id} on epoch {}", self.current_epoch);
        Ok(())
    }

    pub async fn load_proof_id(&mut self) -> eyre::Result<()> {
        let persisted_proof_id =
            persistence::load_persisted_proof_id(&self.persisted_proof_id_dir).await?;

        if let Some(persisted_proof_id) = persisted_proof_id {
            self.last_submitted_proof_id = persisted_proof_id.proof_id;
            tracing::info!(target: "chain-listener", "Loaded persisted proof id {} saved on epoch {}", persisted_proof_id.proof_id, persisted_proof_id.epoch);
            if persisted_proof_id.epoch != self.current_epoch {
                tracing::info!(target: "chain-listener","Persisted proof id epoch is different from current epoch {}, resetting proof id", self.current_epoch);
                self.reset_proof_id().await?;
            }
        } else {
            tracing::info!(target: "chain-listener","No persisted proof id found, starting from zero");
            self.reset_proof_id().await?;
        }

        Ok(())
    }

    pub async fn set_utility_core(&mut self) -> eyre::Result<()> {
        if let Some(ccp_client) = self.ccp_client.as_ref() {
            // We will use the first logical core for utility tasks
            let utility_core = self
                .core_manager
                .get_system_cpu_assignment()
                .logical_core_ids
                .first()
                .cloned()
                .ok_or(eyre::eyre!("No utility core id"))?;

            retry(ExponentialBackoff::default(), || async {
                ccp_client
                    .realloc_utility_cores(vec![utility_core])
                    .await
                    .map_err(|err| {
                        tracing::warn!(target: "chain-listener", "Error reallocating utility core {utility_core} to CCP, error: {err}. Retrying...");
                        eyre::eyre!("Error reallocating utility core {utility_core} to CCP, error: {err}")
                    })?;
                Ok(())
            }).await?;

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
            Ok(Some(status))
        } else {
            Ok(None)
        }
    }

    pub async fn create_ws_client(ws_endpoint: &str) -> Result<WsClient, client::Error> {
        let ws_client = retry(ExponentialBackoff::default(), || async {
            let client = WsClientBuilder::default()
                .build(ws_endpoint)
                .await
                .map_err(|err| {
                    tracing::warn!(
                        target: "chain-listener",
                        "Error connecting to websocket endpoint {}, error: {}; Retrying...",
                        ws_endpoint,
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
            ws_endpoint
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
            self.ws_client =
                ChainListener::create_ws_client(&self.listener_config.ws_endpoint).await?;
        }

        loop {
            let result: Result<(), client::Error> = try {
                self.heads = Some(self.subscribe("newHeads", rpc_params!["newHeads"]).await?);
                self.commitment_activated =
                    Some(self.subscribe("logs", self.cc_activated_params()).await?);
                self.unit_matched = Some(self.subscribe("logs", self.unit_matched_params()).await?);
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
                            ChainListener::create_ws_client(&self.listener_config.ws_endpoint)
                                .await?;
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

    /// Returns active and pending compute units
    async fn refresh_compute_units(&mut self) -> eyre::Result<()> {
        let mut units = self.chain_connector.get_compute_units().await?;

        let in_deal: Vec<_> = units.extract_if(|cu| !cu.deal.is_zero()).collect();

        self.cc_compute_units
            .extend(units.into_iter().map(|unit| (CUID::new(unit.id.0), unit)));

        for cu in in_deal {
            if !cu.deal.is_zero() {
                self.active_deals
                    .insert(cu.deal.to_string().into(), CUID::new(cu.id.0));
            }
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
            self.active_deals.len()
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
                .map(CUID::to_string)
                .collect::<Vec<_>>()
        );

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
                .await.map_err(|err|  {
                if let client::Error::RestartNeeded(_) = err {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to {method}: {err};");
                    Permanent(err)
                } else {
                    tracing::warn!(target: "chain-listener", "Failed to subscribe to {method}: {err}; Retrying...");
                    backoff::Error::transient(err)
                }})
        }).await?;

        Ok(sub)
    }

    fn cc_activated_params(&self) -> ArrayParams {
        let topic = CommitmentActivated::SIGNATURE_HASH.to_string();
        let topics = vec![topic, peer_id_to_hex(self.host_id)];
        rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics": topics})
        ]
    }

    fn unit_activated_params(&self, commitment_id: &CommitmentId) -> ArrayParams {
        let topic = UnitActivated::SIGNATURE_HASH.to_string();
        rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![topic, hex::encode(commitment_id.0)]})
        ]
    }

    fn unit_deactivated_params(&self, commitment_id: &CommitmentId) -> ArrayParams {
        let topic = UnitDeactivated::SIGNATURE_HASH.to_string();
        rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![topic, hex::encode(commitment_id.0)]})
        ]
    }

    fn unit_matched_params(&self) -> ArrayParams {
        let topics = vec![
            ComputeUnitMatched::SIGNATURE_HASH.to_string(),
            peer_id_to_hex(self.host_id),
        ];
        rpc_params![
            "logs",
            json!({"address": self.config.market_contract_address, "topics": topics})
        ]
    }

    async fn process_new_header(
        &mut self,
        event: Option<Result<Value, client::Error>>,
    ) -> eyre::Result<()> {
        let header = event.ok_or(eyre!("Failed to process newHeads event: got None"))?;

        // TODO: add block_number to metrics
        let (block_timestamp, _block_number) = Self::parse_block_header(header?)?;

        // `epoch_number = 1 + (block_timestamp - init_timestamp) / epoch_duration`
        let epoch_number =
            U256::from(1) + (block_timestamp - self.init_timestamp) / self.epoch_duration;
        let epoch_changed = epoch_number > self.current_epoch;

        if epoch_changed {
            // TODO: add epoch_number to metrics
            tracing::info!(target: "chain-listener", "Epoch changed, new epoch number: {epoch_number}");

            // nonce changes every epoch
            self.global_nonce = self.chain_connector.get_global_nonce().await?;
            tracing::info!(target: "chain-listener",
                "New global nonce: {}",
                self.global_nonce
            );

            self.current_epoch = epoch_number;
            tracing::info!(target: "chain-listener", "Resetting proof id counter");
            self.reset_proof_id().await?;
            self.proof_counter.clear();

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

        Ok(())
    }

    async fn process_commitment_activated(
        &mut self,
        event: Option<Result<JsonValue, client::Error>>,
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
        if let Err(err) = self.subscribe_unit_events(&commitment_id).await {
            tracing::warn!(target: "chain-listener", "Failed to subscribe to unit events: {err}");
            self.refresh_subscriptions().await?;
        }

        self.current_commitment = Some(commitment_id);

        self.cc_compute_units = unit_ids
            .into_iter()
            .map(|id| {
                (
                    CUID::new(id.0),
                    ComputeUnit {
                        id,
                        deal: Address::ZERO,
                        startEpoch: cc_event.startEpoch,
                    },
                )
            })
            .collect();

        self.refresh_commitment().await?;

        Ok(())
    }

    async fn process_unit_activated(
        &mut self,
        event: Option<Result<JsonValue, client::Error>>,
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
            },
        );

        self.refresh_commitment().await?;
        Ok(())
    }

    /// Unit goes to Deal
    async fn process_unit_deactivated(
        &mut self,
        event: Option<Result<JsonValue, client::Error>>,
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
        self.acquire_core_for_deal(unit_id)?;
        Ok(())
    }

    pub fn process_unit_matched(
        &mut self,
        event: Option<Result<JsonValue, client::Error>>,
    ) -> eyre::Result<()> {
        let event = event.ok_or(eyre!("Failed to process DealMatched event: got None"))??;
        let log = serde_json::from_value::<Log>(event.clone()).map_err(|err| {
            tracing::error!(target: "chain-listener", "Failed to parse DealMatched event: {err}, data: {event}");
            err
        })?;
        let deal_event = parse_log::<ComputeUnitMatched>(log)?;
        tracing::info!(target: "chain-listener",
            "Received DealMatched event for deal: {}",
            deal_event.deal
        );

        self.active_deals.insert(
            deal_event.deal.to_string().into(),
            CUID::new(deal_event.unitId.0),
        );
        Ok(())
    }

    fn group_cc_units(&self) -> CUGroupResult {
        let mut priority_units: Vec<CUID> = Vec::new();
        let mut non_priority_units: Vec<CUID> = Vec::new();
        let mut pending_units: Vec<CUID> = Vec::new();
        let mut finished_units: Vec<CUID> = Vec::new();
        for (cuid, cu) in &self.cc_compute_units {
            if cu.startEpoch <= self.current_epoch {
                let count = self.proof_counter.get(cuid).unwrap_or(&U256::ZERO);
                if count < &self.min_proofs_per_epoch {
                    priority_units.push(*cuid)
                } else if *count >= self.max_proofs_per_epoch {
                    finished_units.push(*cuid)
                } else {
                    non_priority_units.push(*cuid)
                }
            } else {
                pending_units.push(*cuid);
            }
        }
        CUGroupResult {
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

        let group_result = self.group_cc_units();

        let cc_cores = self.acquire_cores_for_cc(&group_result)?;

        let mut cu_allocation: HashMap<PhysicalCoreId, CUID> = HashMap::new();

        if all_min_proofs_found(&group_result.priority_units) {
            tracing::info!(target: "chain-listener", "All CUs found minimal number of proofs {} in current epoch {}", self.min_proofs_per_epoch, self.current_epoch);
            if all_max_proofs_found(&group_result.non_priority_units) {
                tracing::info!(target: "chain-listener", "All CUs found max number of proofs {} in current epoch {}", self.max_proofs_per_epoch ,self.current_epoch);
                self.stop_commitment().await?;
                return Ok(());
            } else {
                // All CUs were proven, now let's work on submitting proofs for every CU until MAX_PROOF_COUNT is reached
                cu_allocation.extend(
                    cc_cores
                        .non_priority_cores
                        .iter()
                        .cloned()
                        .zip(group_result.non_priority_units.iter().cloned()),
                );

                let mut units = group_result.non_priority_units.iter().cycle();
                // Assign "pending cores" to help generate proofs for "non priority units"
                cc_cores.pending_cores.iter().for_each(|core| {
                    if let Some(unit) = units.next() {
                        cu_allocation.insert(*core, *unit);
                    }
                });
            }
        } else {
            // Use assigned cores to calculate proofs for CUs who haven't reached MIN_PROOF_COUNT yet
            cu_allocation.extend(
                cc_cores
                    .priority_cores
                    .iter()
                    .zip(group_result.priority_units.iter()),
            );

            // Use all spare cores to help CUs to reach MIN_PROOF_COUNT
            let spare_cores: BTreeSet<_> = cc_cores
                .non_priority_cores
                .into_iter()
                .chain(cc_cores.pending_cores.into_iter())
                .chain(cc_cores.finished_cores.into_iter())
                .collect();

            let mut units = group_result.priority_units.iter().cycle();
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

        ccp_client
            .on_active_commitment(
                self.global_nonce,
                self.difficulty,
                cu_allocation,
            )
            .await
            .map_err(|err| {
                tracing::error!(target: "chain-listener", "Failed to send commitment to CCP: {err}");
                eyre::eyre!("Failed to send commitment to CCP: {err}")
            })?;

        Ok(())
    }

    fn acquire_cores_for_cc(
        &self,
        cu_group_result: &CUGroupResult,
    ) -> eyre::Result<PhysicalCoreGroupResult> {
        let mut units = cu_group_result.priority_units.clone();
        units.extend(cu_group_result.non_priority_units.clone());
        units.extend(cu_group_result.pending_units.clone());
        units.extend(cu_group_result.finished_units.clone());

        let cores = self.core_manager.acquire_worker_core(AcquireRequest::new(
            units.to_vec(),
            WorkType::CapacityCommitment,
        ));

        fn filter(units: &Vec<CUID>, assignment: &Assignment) -> Vec<PhysicalCoreId> {
            units
                .into_iter()
                .filter_map(|cuid| {
                    assignment
                        .cuid_core_data
                        .get(&cuid)
                        .map(|data| data.physical_core_id)
                })
                .collect()
        }

        match cores {
            Ok(assignment) => {
                let priority_units = filter(&cu_group_result.priority_units, &assignment);
                let non_priority_units = filter(&cu_group_result.non_priority_units, &assignment);
                let pending_units = filter(&cu_group_result.pending_units, &assignment);
                let finished_units = filter(&cu_group_result.finished_units, &assignment);

                Ok(PhysicalCoreGroupResult {
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
                tracing::warn!("Found {required} CUs in the Capacity Commitment, but Nox has only {available} Cores available for CC");
                let assign_units = units.iter().take(available).cloned().collect();
                self.core_manager.release(units);
                let assignment = self.core_manager.acquire_worker_core(AcquireRequest::new(
                    assign_units,
                    WorkType::CapacityCommitment,
                ))?;
                let priority_units = filter(&cu_group_result.priority_units, &assignment);
                let non_priority_units = filter(&cu_group_result.non_priority_units, &assignment);
                let pending_units = filter(&cu_group_result.pending_units, &assignment);
                let finished_units = filter(&cu_group_result.finished_units, &assignment);

                Ok(PhysicalCoreGroupResult {
                    priority_cores: priority_units,
                    non_priority_cores: non_priority_units,
                    pending_cores: pending_units,
                    finished_cores: finished_units,
                })
            }
        }
    }

    fn acquire_core_for_deal(&self, unit_id: CUID) -> eyre::Result<()> {
        self.core_manager
            .acquire_worker_core(AcquireRequest::new(vec![unit_id], WorkType::Deal))?;
        Ok(())
    }

    /// Should be called only if Commitment is Inactive, Failed, Removed or not exists
    async fn reset_commitment(&mut self) -> eyre::Result<()> {
        self.cc_compute_units.clear();
        self.active_deals.clear();
        self.current_commitment = None;
        self.stop_commitment().await?;
        Ok(())
    }

    async fn stop_commitment(&self) -> eyre::Result<()> {
        tracing::info!(target: "chain-listener", "Stopping current commitment");
        if let Some(ref ccp_client) = self.ccp_client {
            ccp_client.on_no_active_commitment().await.map_err(|err| {
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

        let result_hash = ResultHash::from_slice(*self.difficulty.as_ref());

        // proof_id is used only by CCP and is not sent to chain
        let proof_id = CCProofId::new(self.global_nonce, self.difficulty, ProofIdx::zero());
        let units = self.cc_compute_units.keys().cloned().collect::<Vec<_>>();
        for unit in units {
            let local_nonce = LocalNonce::random();
            self.submit_proof(CCProof::new(proof_id, local_nonce, unit, result_hash))
                .await?;
        }

        Ok(())
    }

    async fn poll_proofs(&mut self) -> eyre::Result<()> {
        if self.current_commitment.is_none() || self.cc_compute_units.is_empty() {
            return Ok(());
        }

        if let Some(ref ccp_client) = self.ccp_client {
            tracing::trace!(target: "chain-listener", "Polling proofs after: {}", self.last_submitted_proof_id);

            let proofs = ccp_client
                .get_proofs_after(self.last_submitted_proof_id, PROOF_POLL_LIMIT)
                .await?;

            // TODO: send only in batches

            // Filter proofs related to current epoch only
            let proofs: Vec<_> = proofs
                .into_iter()
                .filter(|p| p.id.global_nonce == self.global_nonce)
                .collect();

            if proofs.len() > 0 {
                tracing::info!(target: "chain-listener", "Found {} proofs from polling", proofs.len());
            }

            for proof in proofs.into_iter() {
                let id = proof.id.idx;
                tracing::info!(target: "chain-listener", "Submitting proof: {id}");
                self.submit_proof(proof).await?;
                self.set_proof_id(proof.id.idx).await?;
            }
        }
        Ok(())
    }

    async fn submit_proof(&mut self, proof: CCProof) -> eyre::Result<()> {
        if !self.cc_compute_units.contains_key(&proof.cu_id) {
            return Ok(());
        }

        let submit = retry(ExponentialBackoff::default(), || async {
            self.chain_connector.submit_proof(proof).await.map_err(|err| {
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
                            tracing::info!(target: "chain-listener", "Too many proofs found for compute unit {}", proof.cu_id);

                            self.proof_counter
                                .insert(proof.cu_id, self.max_proofs_per_epoch);
                            self.refresh_commitment().await?;

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
                            tracing::error!(target: "chain-listener", "Failed to submit proof {err}");
                            tracing::error!(target: "chain-listener", "Proof {:?} ", proof);
                            // In case of contract errors we just skip these proofs and continue
                            Ok(())
                        }
                    }
                    _ => {
                        tracing::error!(target: "chain-listener", "Failed to submit proof: {err}");
                        tracing::error!(target: "chain-listener", "Proof {:?} ", proof);

                        Err(err.into())
                    }
                }
            }
            Ok(tx_id) => {
                tracing::info!(target: "chain-listener", "Submitted proof {}, txHash: {tx_id}", proof.id.idx);
                self.pending_proof_txs.push((tx_id, proof.cu_id));
                Ok(())
            }
        }
    }

    fn parse_block_header(header: Value) -> eyre::Result<(U256, U256)> {
        let obj = header.as_object().ok_or(eyre::eyre!(
            "newHeads: header is not an object; got {header}"
        ))?;

        let timestamp = obj
            .get("timestamp")
            .and_then(Value::as_str)
            .ok_or(eyre::eyre!(
                "newHeads: timestamp field not found; got {header}"
            ))?
            .to_string();

        let block_number = header
            .get("number")
            .and_then(Value::as_str)
            .ok_or(eyre::eyre!(
                "newHeads: number field not found; got {header}"
            ))?
            .to_string();

        Ok((U256::from_str(&timestamp)?, U256::from_str(&block_number)?))
    }

    async fn poll_deal_statuses(&mut self) -> eyre::Result<()> {
        if self.active_deals.is_empty() {
            return Ok(());
        }

        let statuses = retry(ExponentialBackoff::default(), || async {
            let s = self.chain_connector.get_deal_statuses(self.active_deals.keys()).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to poll deal statuses: {err}; Retrying...");
                eyre!("Failed to poll deal statuses: {err}; Retrying...")
            })?;

            Ok(s)
        })
        .await?;

        for (status, (deal_id, cu_id)) in statuses
            .into_iter()
            .zip(self.active_deals.clone().into_iter())
        {
            match status {
                Ok(status) => match status {
                    Deal::Status::INSUFFICIENT_FUNDS | Deal::Status::ENDED => {
                        tracing::info!(target: "chain-listener", "Deal {deal_id} status: {status:?}; Exiting...");
                        self.exit_deal(&deal_id, cu_id).await?;
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

    async fn exit_deal(&mut self, deal_id: &DealId, cu_id: CUID) -> eyre::Result<()> {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(3)),
            ..ExponentialBackoff::default()
        };

        retry(backoff, || async {
            self.chain_connector.exit_deal(&cu_id).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to exit deal {deal_id}: {err}");
                eyre!("Failed to exit deal {deal_id}: {err}; Retrying...")
            })?;
            Ok(())
        })
        .await?;

        self.active_deals.remove(deal_id);
        Ok(())
    }

    async fn poll_pending_proof_txs(&mut self) -> eyre::Result<()> {
        if self.pending_proof_txs.is_empty() {
            return Ok(());
        }

        let statuses = retry(ExponentialBackoff::default(), || async {
            let s = self.chain_connector.get_tx_statuses(self.pending_proof_txs.iter().map(|(tx, _)| tx)).await.map_err(|err| {
                tracing::warn!(target: "chain-listener", "Failed to poll pending proof txs statuses: {err}");
                eyre!("Failed to poll pending proof txs statuses: {err}; Retrying...")
            })?;

            Ok(s)
        })
            .await?;

        let mut refresh_neeeded = false;
        let mut stats_updated = false;
        for (status, (tx_hash, cu_id)) in statuses
            .into_iter()
            .zip(self.pending_proof_txs.clone().into_iter())
        {
            match status {
                Ok(Some(status)) => {
                    if status {
                        tracing::info!(target: "chain-listener", "Proof tx {tx_hash} confirmed");
                        stats_updated = true;
                        let counter = self
                            .proof_counter
                            .entry(cu_id)
                            .and_modify(|c| {
                                *c = c.add(Uint::from(1));
                            })
                            .or_insert(Uint::from(1));

                        if *counter >= self.min_proofs_per_epoch {
                            tracing::info!(target: "chain-listener", "Compute unit {cu_id} submitted enough proofs");
                            refresh_neeeded = true;
                        }
                    } else {
                        tracing::warn!(target: "chain-listener", "Proof tx {tx_hash} not confirmed");
                    }

                    self.pending_proof_txs.retain(|(tx, _)| tx != &tx_hash);
                }
                Ok(None) => {
                    tracing::debug!(target: "chain-listener", "Proof tx {tx_hash} not found");
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
            tracing::info!(target: "chain-listener", "Confirmed proofs count: {:?}", self.proof_counter.iter().map(|(cu, count)| format!("{}: {}", cu, count.to_string())).collect::<Vec<_>>());
        }

        Ok(())
    }
}

struct CUGroupResult {
    pub priority_units: Vec<CUID>,
    pub non_priority_units: Vec<CUID>,
    pub pending_units: Vec<CUID>,
    pub finished_units: Vec<CUID>,
}

struct PhysicalCoreGroupResult {
    pub priority_cores: Vec<PhysicalCoreId>,
    pub non_priority_cores: Vec<PhysicalCoreId>,
    pub pending_cores: Vec<PhysicalCoreId>,
    pub finished_cores: Vec<PhysicalCoreId>,
}

fn all_min_proofs_found(priority_units: &[CUID]) -> bool {
    priority_units.is_empty()
}

fn all_max_proofs_found(non_priority_units: &[CUID]) -> bool {
    non_priority_units.is_empty()
}
