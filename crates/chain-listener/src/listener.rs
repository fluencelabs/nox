use backoff::Error::Permanent;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use backoff::future::retry;
use backoff::ExponentialBackoff;
use ccp_rpc_client::{CCPRpcHttpClient, OrHex};
use ccp_shared::proof::{CCProof, CCProofId, ProofIdx};
use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash};
use cpu_utils::PhysicalCoreId;
use ethabi::ethereum_types::U256;
use eyre::eyre;
use jsonrpsee::core::client::{Client as WsClient, Error, Subscription, SubscriptionClientT};
use jsonrpsee::core::{client, JsonValue};
use jsonrpsee::rpc_params;
use libp2p_identity::PeerId;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use chain_connector::{ChainConnector, ConnectorError};
use chain_data::{parse_log, peer_id_to_hex, ChainData, Log};
use chain_types::{
    CommitmentId, CommitmentStatus, ComputeUnit, DealStatus, PendingUnit, COMMITMENT_IS_NOT_ACTIVE,
    PEER_NOT_EXISTS, TOO_MANY_PROOFS,
};
use core_manager::manager::{CoreManager, CoreManagerFunctions};
use core_manager::types::{AcquireRequest, WorkType};
use core_manager::CUID;
use server_config::{ChainConfig, ChainListenerConfig};
use types::DealId;

use crate::event::cc_activated::CommitmentActivated;
use crate::event::{
    CommitmentActivatedData, DealMatched, DealMatchedData, UnitActivated, UnitActivatedData,
    UnitDeactivated, UnitDeactivatedData,
};
use crate::persistence;

const PROOF_POLL_LIMIT: usize = 10;

pub struct ChainListener {
    config: ChainConfig,

    chain_connector: Arc<ChainConnector>,
    ws_client: WsClient,
    ccp_client: Option<CCPRpcHttpClient>,
    core_manager: Arc<CoreManager>,

    timer_resolution: Duration,
    host_id: PeerId,

    difficulty: Difficulty,
    init_timestamp: U256,
    global_nonce: GlobalNonce,
    current_epoch: U256,
    epoch_duration: U256,

    // proof_counter: HashMap<CUID, u64>,
    current_commitment: Option<CommitmentId>,

    active_compute_units: BTreeSet<CUID>,
    pending_compute_units: BTreeSet<PendingUnit>,

    active_deals: BTreeMap<DealId, CUID>,

    /// Resets every epoch
    last_submitted_proof_id: ProofIdx,
    persisted_proof_id_dir: PathBuf,

    unit_activated: Option<Subscription<Log>>,
    unit_deactivated: Option<Subscription<Log>>,
    heads: Option<Subscription<JsonValue>>,
    commitment_activated: Option<Subscription<Log>>,
    unit_matched: Option<Subscription<Log>>,
}

async fn poll_subscription<T>(s: &mut Option<Subscription<T>>) -> Option<Result<T, client::Error>>
where
    T: DeserializeOwned,
{
    match s {
        Some(ref mut s) => s.next().await,
        None => None,
    }
}

impl ChainListener {
    pub fn new(
        chain_config: ChainConfig,
        listener_config: ChainListenerConfig,
        host_id: PeerId,
        chain_connector: Arc<ChainConnector>,
        core_manager: Arc<CoreManager>,
        ws_client: WsClient,
        ccp_client: Option<CCPRpcHttpClient>,
        persisted_proof_id_dir: PathBuf,
    ) -> Self {
        if ccp_client.is_none() {
            tracing::warn!(target: "chain-listener", "CCP client is not set, will submit mocked proofs");
        }

        Self {
            chain_connector,
            ws_client,
            config: chain_config,
            host_id,
            difficulty: Difficulty::default(),
            init_timestamp: U256::zero(),
            global_nonce: GlobalNonce::new([0; 32]).into(),
            current_epoch: U256::zero(),
            epoch_duration: U256::zero(),
            current_commitment: None,
            active_compute_units: BTreeSet::new(),
            pending_compute_units: BTreeSet::new(),
            core_manager,
            timer_resolution: listener_config.proof_poll_period,
            ccp_client,
            last_submitted_proof_id: ProofIdx::zero(),
            persisted_proof_id_dir,
            unit_activated: None,
            unit_deactivated: None,
            heads: None,
            commitment_activated: None,
            unit_matched: None,
            active_deals: BTreeMap::new(),
        }
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

        tracing::info!(target: "chain-listener","Commitment initial params: difficulty {}, global nonce {}, init_timestamp {}, epoch_duration {}, current_epoch {}",  init_params.difficulty, init_params.global_nonce, init_params.init_timestamp, init_params.epoch_duration, init_params.current_epoch);
        Ok(())
    }

    async fn refresh_compute_units(&mut self) -> eyre::Result<()> {
        loop {
            let result: eyre::Result<()> = try {
                self.refresh_commitment_params().await?;

                let (active, pending) = self.get_compute_units().await?;
                self.refresh_current_commitment_id().await?;

                if let Some(ref c) = self.current_commitment {
                    tracing::info!(target: "chain-listener", "Current commitment id: {}", c);
                    tracing::info!(target: "chain-listener", "Subscribing to unit events");
                    self.subscribe_unit_deactivated().await?;
                    self.subscribe_unit_activated().await?;
                    tracing::info!(target: "chain-listener", "Successfully subscribed to unit events");
                } else {
                    tracing::info!(target: "chain-listener", "Compute peer has no commitment");
                    self.stop_commitment().await?
                }

                if let Some(status) = self.get_commitment_status().await? {
                    tracing::info!(target: "chain-listener", "Current commitment status: {status}");

                    match status {
                        CommitmentStatus::Active => {
                            self.active_compute_units.extend(active);
                            self.pending_compute_units.extend(pending);
                            self.refresh_commitment().await?;
                        }

                        CommitmentStatus::Inactive
                        | CommitmentStatus::Failed
                        | CommitmentStatus::Removed => {
                            self.reset_commitment().await?;
                        }
                        CommitmentStatus::WaitDelegation | CommitmentStatus::WaitStart => {
                            tracing::info!(target: "chain-listener", "Waiting for commitment to be activated; Stopping current one");
                            self.pending_compute_units.extend(pending);
                            self.stop_commitment().await?
                        }
                    }
                }

                ()
            };

            if let Err(e) = result {
                tracing::error!(target: "chain-listener", "Failed to refresh compute units: {e}");
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
        let mut backoff = ExponentialBackoff::default();
        backoff.max_elapsed_time = Some(Duration::from_secs(3));

        let write = retry(backoff, || async {
            persistence::persist_proof_id(
                &self.persisted_proof_id_dir,
                self.last_submitted_proof_id,
                self.current_epoch,
            ).await.map_err(|err|{
                tracing::error!(target: "chain-listener", "Failed to persist proof id: {err}; Retrying...");
                eyre!(err)
            })?;
            Ok(())
        }).await;

        if let Err(err) = write {
            tracing::error!(target: "chain-listener", "Failed to persist proof id: {err}; Ignoring..");
        }

        self.last_submitted_proof_id = proof_id;
        tracing::info!(target: "chain-listener", "Persisted proof id {proof_id} on epoch {}", self.current_epoch);
        Ok(())
    }

    pub async fn load_proof_id(&mut self) -> eyre::Result<()> {
        let persisted_proof_id =
            persistence::load_persisted_proof_id(&self.persisted_proof_id_dir).await?;

        if let Some(persisted_proof_id) = persisted_proof_id {
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
                    .realloc_utility_core(vec![utility_core])
                    .await
                    .map_err(|err| {
                        tracing::error!(target: "chain-listener", "Error reallocating utility core {utility_core} to CCP, error: {err}. Retrying...");
                        eyre::eyre!("Error reallocating utility core {utility_core} to CCP, error: {err}")
                    })?;
                Ok(())
            }).await?;

            tracing::info!("Utility core {utility_core} successfully reallocated");
        }
        Ok(())
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
                if let Err(err) = self.subscribe_new_heads().await {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to newHeads: {err}; Stopping...");
                    exit(1);
                }

                if let Err(err) =  self.subscribe_cc_activated().await {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to CommitmentActivated event: {err}; Stopping...");
                    exit(1);
                }

                if let Err(err) = self.subscribe_deal_matched().await {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to deal matched: {err}; Stopping...");
                    exit(1);
                }

                tracing::info!(target: "chain-listener", "Subscribed successfully");

                let setup: eyre::Result<()> = try {
                    self.refresh_compute_units().await?;
                    self.load_proof_id().await?;
                };
                if let Err(err) = setup {
                    tracing::error!(target: "chain-listener", "ChainListener: compute units refresh error: {err}");
                    panic!("ChainListener startup error: {err}");
                }

                let mut timer = IntervalStream::new(interval(self.timer_resolution));

                loop {
                    tokio::select! {
                        event = poll_subscription(&mut self.heads) => {
                            if let Some(header) = event {
                                if let Err(err) = self.process_new_header(header).await {
                                   tracing::error!(target: "chain-listener", "newHeads event processing error: {err}");
                                    if let Err(err) = self.subscribe_new_heads().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to newHeads: {err}; Stopping...");
                                        exit(1);
                                    }
                                }
                            } else {
                                if let Err(err) = self.subscribe_new_heads().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to newHeads: {err}; Stopping...");
                                        exit(1);
                                }
                            }
                        },
                        event = poll_subscription(&mut self.commitment_activated) => {
                            if let Some(cc) = event {
                                if let Err(err) = self.process_commitment_activated(cc).await {
                                    tracing::error!(target: "chain-listener", "CommitmentActivated event processing error: {err}");
                                    if let Err(err) =  self.subscribe_cc_activated().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to CommitmentActivated event: {err}; Stopping...");
                                        exit(1);
                                    }
                                }
                            } else {
                                if let Err(err) =  self.subscribe_cc_activated().await {
                                    tracing::error!(target: "chain-listener", "Failed to resubscribe to CommitmentActivated event: {err}; Stopping...");
                                    exit(1);
                                }
                            }
                        },
                        event = poll_subscription(&mut self.unit_activated) => {
                            if let Some(event) = event {
                                if let Err(err) = self.process_unit_activated(event).await {
                                   tracing::error!(target: "chain-listener", "UnitActivated event processing error: {err}");
                                    if let Err(err) = self.subscribe_unit_activated().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to UnitActivated: {err}; Stopping...");
                                        exit(1);
                                    }
                                }
                            } else {
                                 if let Err(err) = self.subscribe_unit_activated().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to UnitActivated: {err}; Stopping...");
                                        exit(1);
                                }
                            }
                        },
                        event = poll_subscription(&mut self.unit_deactivated) => {
                            if let Some(event) = event {
                                if let Err(err) = self.process_unit_deactivated(event).await {
                                    tracing::error!(target: "chain-listener", "UnitDeactivated event processing error: {err}");
                                    if let Err(err) = self.subscribe_unit_deactivated().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to UnitDeactivated: {err}; Stopping...");
                                        exit(1);
                                    }
                                }
                            } else {
                                if let Err(err) = self.subscribe_unit_deactivated().await {
                                    tracing::error!(target: "chain-listener", "Failed to resubscribe to UnitDeactivated: {err}; Stopping...");
                                    exit(1);
                                }
                            }
                        },
                        event = poll_subscription(&mut self.unit_matched) => {
                            if let Some(event) = event {
                                if let Err(err) = self.process_deal_matched(event) {
                                    tracing::error!(target: "chain-listener", "DealMatched event processing error: {err}");
                                    if let Err(err) = self.subscribe_deal_matched().await {
                                        tracing::error!(target: "chain-listener", "Failed to resubscribe to DealMatched: {err}; Stopping...");
                                        exit(1);
                                    }
                                }
                            } else {
                                if let Err(err) = self.subscribe_deal_matched().await {
                                    tracing::error!(target: "chain-listener", "Failed to resubscribe to DealMatched: {err}; Stopping...");
                                    exit(1);
                                }
                            }
                        },
                        _ = timer.next() => {
                            if self.ccp_client.is_some() {
                                if let Err(err) = self.poll_proofs().await {
                                    tracing::error!(target: "chain-listener", "Failed to poll/submit proofs: {err}");
                                }
                            } else{
                                if let Err(err) = self.submit_mocked_proofs().await {
                                    tracing::error!(target: "chain-listener", "Failed to submit mocked proofs: {err}");
                                }
                             }

                            if let Err(err) = self.poll_deal_statuses().await {
                                tracing::error!(target: "chain-listener", "Failed to poll deal statuses: {err}");
                            }
                        }
                    }
                }
            })
            .expect("Could not spawn task");

        result
    }

    async fn get_commitment_status(&self) -> eyre::Result<Option<CommitmentStatus>> {
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

    /// Returns active and pending compute units
    async fn get_compute_units(&mut self) -> eyre::Result<(Vec<CUID>, Vec<PendingUnit>)> {
        let mut units = self.chain_connector.get_compute_units().await?;

        let in_deal: Vec<_> = units.extract_if(|cu| cu.deal.is_some()).collect();

        let (active, pending): (Vec<ComputeUnit>, Vec<ComputeUnit>) = units
            .into_iter()
            .partition(|unit| unit.start_epoch <= self.current_epoch);

        let active: Vec<_> = active.into_iter().map(|unit| unit.id).collect();
        let pending: Vec<PendingUnit> = pending.into_iter().map(PendingUnit::from).collect();

        tracing::info!(target: "chain-listener",
            "Compute units mapping: active {}, pending {}, in deal {}",
            active.len(),
            pending.len(),
            in_deal.len()
        );

        // TODO: log compute units pretty
        tracing::info!(target: "chain-listener",
            "Active compute units: {:?}",
            active.iter().map(CUID::to_string).collect::<Vec<_>>()
        );
        tracing::info!(target: "chain-listener",
            "Pending compute units: {:?}",
            pending
                .iter()
                .map(|cu| cu.id.to_string())
                .collect::<Vec<_>>()
        );
        tracing::info!(target: "chain-listener",
            "In deal compute units: {:?}",
            in_deal
                .iter()
                .map(|cu| cu.id.to_string())
                .collect::<Vec<_>>()
        );

        for cu in in_deal {
            if let Some(deal) = cu.deal {
                self.active_deals.insert(deal, cu.id);
            }
        }

        Ok((active, pending))
    }

    async fn subscribe_new_heads(&mut self) -> eyre::Result<()> {
        let sub = retry(ExponentialBackoff::default(), || async {
            let subs = self
                .ws_client
                .subscribe("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe").await.map_err(|err| {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to newHeads: {err}; Retrying...");
                    eyre!(err)
                })?;

            Ok(subs)
        })
        .await?;

        self.heads = Some(sub);
        Ok(())
    }

    async fn subscribe_cc_activated(&mut self) -> eyre::Result<()> {
        let sub = retry(ExponentialBackoff::default(), || async {
            let topics = vec![
                CommitmentActivatedData::topic(),
                peer_id_to_hex(self.host_id),
            ];
            let params = rpc_params![
                "logs",
                json!({"address": self.config.cc_contract_address, "topics": topics})
            ];
             let subs = self
                .ws_client
                .subscribe("eth_subscribe", params, "eth_unsubscribe")
                .await.map_err(|err| {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to cc events: {err}; Retrying...");
                    eyre!(err)
                })?;

            Ok(subs)
        })
        .await?;

        self.commitment_activated = Some(sub);
        Ok(())
    }

    async fn subscribe_unit_activated(&mut self) -> eyre::Result<()> {
        if let Some(c_id) = self.current_commitment.as_ref() {
            let sub = retry(ExponentialBackoff::default(), || async {
              let params = rpc_params!["logs",
                json!({"address": self.config.cc_contract_address, "topics":  vec![UnitActivatedData::topic(), hex::encode(&c_id.0)]})];
                let subs = self
                    .ws_client
                    .subscribe("eth_subscribe", params, "eth_unsubscribe")
                    .await.map_err(|err| {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to unit activated: {err}; Retrying...");
                    eyre!(err)
                })?;

                Ok(subs)
            })
                .await?;

            self.unit_activated = Some(sub);
        }

        Ok(())
    }

    async fn subscribe_unit_deactivated(&mut self) -> eyre::Result<()> {
        if let Some(c_id) = self.current_commitment.as_ref() {
            let sub = retry(ExponentialBackoff::default(), || async {
                let params = rpc_params![
                    "logs",
                    json!({"address": self.config.cc_contract_address, "topics":  vec![UnitDeactivatedData::topic(), hex::encode(&c_id.0)]})
                ];
                let subs = self
                    .ws_client
                    .subscribe("eth_subscribe", params, "eth_unsubscribe")
                    .await.map_err(|err| {
                        tracing::error!(target: "chain-listener", "Failed to subscribe to unit deactivated: {err}; Retrying...");
                        eyre!(err)
                    })?;

                Ok(subs)
            }).await?;

            self.unit_deactivated = Some(sub);
        }

        Ok(())
    }

    async fn subscribe_deal_matched(&mut self) -> eyre::Result<()> {
        let sub = retry(ExponentialBackoff::default(), || async {
            let topics = vec![
                DealMatchedData::topic(),
                peer_id_to_hex(self.host_id),
            ];
            let params = rpc_params![
                "logs",
                json!({"address": self.config.market_contract_address, "topics": topics})
            ];
            let subs = self
                .ws_client
                .subscribe("eth_subscribe", params, "eth_unsubscribe")
                .await.map_err(|err| {
                    tracing::error!(target: "chain-listener", "Failed to subscribe to deal matched: {err}; Retrying...");
                    eyre!(err)
                })?;

            Ok(subs)
        }).await?;

        self.unit_matched = Some(sub);
        Ok(())
    }

    async fn process_new_header(&mut self, event: Result<Value, Error>) -> eyre::Result<()> {
        // TODO: add block_number to metrics
        let (block_timestamp, _block_number) = Self::parse_block_header(event?)?;

        // `epoch_number = 1 + (block_timestamp - init_timestamp) / epoch_duration`
        let epoch_number =
            U256::from(1) + (block_timestamp - self.init_timestamp) / self.epoch_duration;
        let epoch_changed = epoch_number > self.current_epoch;

        if epoch_changed {
            // TODO: add epoch_number to metrics
            tracing::info!(target: "chain-listener", "Epoch changed, new epoch number: {epoch_number}");

            tracing::info!(target: "chain-listener", "Resetting proof id counter");
            self.reset_proof_id().await?;

            // nonce changes every epoch
            self.global_nonce = self.chain_connector.get_global_nonce().await?;
            tracing::info!(target: "chain-listener",
                "New global nonce: {}",
                self.global_nonce
            );

            if let Some(status) = self.get_commitment_status().await? {
                tracing::info!(target: "chain-listener", "Current commitment status: {status}");

                match status {
                    CommitmentStatus::Active => {
                        self.activate_pending_units(epoch_number).await?;
                    }
                    CommitmentStatus::Inactive
                    | CommitmentStatus::Failed
                    | CommitmentStatus::Removed => {
                        self.reset_commitment().await?;
                    }
                    CommitmentStatus::WaitDelegation => {}
                    CommitmentStatus::WaitStart => {}
                }
            }

            self.current_epoch = epoch_number;
        }

        Ok(())
    }

    async fn process_commitment_activated(
        &mut self,
        event: Result<Log, Error>,
    ) -> eyre::Result<()> {
        let cc_event = parse_log::<CommitmentActivatedData, CommitmentActivated>(event?)?;
        let unit_ids = cc_event.info.unit_ids;
        tracing::info!(target: "chain-listener",
            "Received CommitmentActivated event for commitment: {}, startEpoch: {}, unitIds: {:?}",
            cc_event.info.commitment_id,
            cc_event.info.start_epoch,
            unit_ids
                .iter()
                .map(CUID::to_string)
                .collect::<Vec<_>>()
        );

        self.current_commitment = Some(cc_event.info.commitment_id);

        self.subscribe_unit_activated().await?;
        self.subscribe_unit_deactivated().await?;

        let is_cc_active = cc_event.info.start_epoch <= self.current_epoch;

        if is_cc_active {
            self.active_compute_units = unit_ids.into_iter().collect();
            self.refresh_commitment().await?;
        } else {
            self.pending_compute_units = unit_ids
                .into_iter()
                .map(|id| PendingUnit::new(id, cc_event.info.start_epoch))
                .collect();
            self.stop_commitment().await?;
        }

        Ok(())
    }

    async fn process_unit_activated(&mut self, event: Result<Log, Error>) -> eyre::Result<()> {
        let unit_event = parse_log::<UnitActivatedData, UnitActivated>(event?)?;
        tracing::info!(target: "chain-listener",
            "Received UnitActivated event for unit: {}, startEpoch: {}",
            unit_event.info.unit_id,
            unit_event.info.start_epoch
        );

        if self.current_epoch >= unit_event.info.start_epoch {
            self.active_compute_units.insert(unit_event.info.unit_id);
            self.refresh_commitment().await?;
        } else {
            // Will be activated on the `start_epoch`
            self.pending_compute_units.insert(unit_event.info.into());
        }
        Ok(())
    }

    /// Unit goes to Deal
    async fn process_unit_deactivated(
        &mut self,
        event: Result<Log, client::Error>,
    ) -> eyre::Result<()> {
        let unit_event = parse_log::<UnitDeactivatedData, UnitDeactivated>(event?)?;

        tracing::info!(target: "chain-listener",
            "Received UnitDeactivated event for unit: {}",
            unit_event.info.unit_id
        );
        self.active_compute_units.remove(&unit_event.info.unit_id);
        self.pending_compute_units
            .retain(|cu| cu.id != unit_event.info.unit_id);
        self.refresh_commitment().await?;
        self.acquire_deal_core(unit_event.info.unit_id)?;
        Ok(())
    }

    pub fn process_deal_matched(&mut self, event: Result<Log, client::Error>) -> eyre::Result<()> {
        let deal_event = parse_log::<DealMatchedData, DealMatched>(event?)?;
        tracing::info!(target: "chain-listener",
            "Received DealMatched event for deal: {}",
            deal_event.info.deal_id
        );

        self.active_deals
            .insert(deal_event.info.deal_id, deal_event.info.unit_id);
        Ok(())
    }

    /// Send GlobalNonce, Difficulty and Core<>CUID mapping (full commitment info) to CCP
    async fn refresh_commitment(&self) -> eyre::Result<()> {
        if self.active_compute_units.is_empty() {
            self.stop_commitment().await?;
            return Ok(());
        }

        tracing::info!(target: "chain-listener",
            "Refreshing commitment, active compute units: {}",
            self.active_compute_units
                .iter()
                .map(CUID::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
        tracing::info!(target: "chain-listener", "Global nonce: {}", self.global_nonce);
        tracing::info!(target: "chain-listener", "Difficulty: {}", self.difficulty);
        if let Some(ref ccp_client) = self.ccp_client {
            let cores = self.acquire_active_units()?;
            ccp_client
                .on_active_commitment(
                    OrHex::from(self.global_nonce),
                    OrHex::from(self.difficulty),
                    cores,
                )
                .await
                .map_err(|err| {
                    tracing::error!(target: "chain-listener", "Failed to send commitment to CCP: {err}");
                    eyre::eyre!("Failed to send commitment to CCP: {err}")
                })?;
        }
        Ok(())
    }

    fn acquire_active_units(&self) -> eyre::Result<HashMap<PhysicalCoreId, OrHex<CUID>>> {
        let cores = self
            .core_manager
            .acquire_worker_core(AcquireRequest::new(
                self.active_compute_units.clone().into_iter().collect(),
                WorkType::CapacityCommitment,
            ))
            .map_err(|err| {
                tracing::error!(target: "chain-listener", "Failed to acquire cores for active units: {err}");
                eyre::eyre!("Failed to acquire cores for active units: {err}")
            })?;

        Ok(cores
            .physical_core_ids
            .into_iter()
            .zip(
                self.active_compute_units
                    .clone()
                    .into_iter()
                    .map(OrHex::Data),
            )
            .collect())
    }

    fn acquire_deal_core(&self, unit_id: CUID) -> eyre::Result<()> {
        self.core_manager
            .acquire_worker_core(AcquireRequest::new(vec![unit_id], WorkType::Deal))?;
        Ok(())
    }

    async fn reset_commitment(&mut self) -> eyre::Result<()> {
        self.active_compute_units.clear();
        self.pending_compute_units.clear();
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

    async fn activate_pending_units(&mut self, new_epoch: U256) -> eyre::Result<()> {
        let to_activate: Vec<_> = self
            .pending_compute_units
            .extract_if(|unit| unit.start_epoch <= new_epoch)
            .map(|cu| cu.id)
            .collect();

        tracing::info!(target: "chain-listener",
            "Activating pending compute units: [{}]",
            to_activate
                .iter()
                .map(CUID::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.active_compute_units.extend(to_activate);
        self.refresh_commitment().await?;
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
        for unit in self.active_compute_units.clone().into_iter() {
            let local_nonce = LocalNonce::random();
            self.submit_proof(CCProof::new(proof_id, local_nonce, unit, result_hash))
                .await?;
        }

        Ok(())
    }

    async fn poll_proofs(&mut self) -> eyre::Result<()> {
        if self.current_commitment.is_none() {
            return Ok(());
        }

        if let Some(ref ccp_client) = self.ccp_client {
            tracing::info!(target: "chain-listener", "Polling proofs after: {}", self.last_submitted_proof_id);

            let proofs = ccp_client
                .get_proofs_after(self.last_submitted_proof_id, PROOF_POLL_LIMIT)
                .await?;

            // TODO: send only in batches

            // Filter proofs related to current epoch only
            let proofs: Vec<_> = proofs
                .into_iter()
                .filter(|p| p.id.global_nonce == self.global_nonce)
                .collect();

            tracing::info!(target: "chain-listener", "Found {} proofs from polling", proofs.len());
            for proof in proofs.into_iter() {
                let id = proof.id.idx.clone();
                tracing::info!(target: "chain-listener", "Submitting proof: {id}");
                self.submit_proof(proof).await?;
                self.set_proof_id(proof.id.idx).await?;
            }
        }
        Ok(())
    }

    async fn submit_proof(&mut self, proof: CCProof) -> eyre::Result<()> {
        if !self.active_compute_units.contains(&proof.cu_id) {
            return Ok(());
        }

        let submit = retry(ExponentialBackoff::default(), || async {
            self.chain_connector.submit_proof(proof).await.map_err(|err| {
                match err {
                    ConnectorError::RpcCallError { .. } => { Permanent(err) }
                   _ => {
                        tracing::error!(target: "chain-listener", "Failed to submit proof: {err}. Retrying..");
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
                        if data.contains(TOO_MANY_PROOFS) {
                            tracing::info!(target: "chain-listener", "Too many proofs found for compute unit {}, stopping until next epoch", proof.cu_id);

                            // TODO: acquire core for other units to help with proofs calculation

                            self.active_compute_units.remove(&proof.cu_id);
                            self.pending_compute_units
                                .insert(PendingUnit::new(proof.cu_id, self.current_epoch + 1));
                            self.refresh_commitment().await?;

                            Ok(())
                        } else if data.contains(COMMITMENT_IS_NOT_ACTIVE) {
                            tracing::info!(target: "chain-listener", "Submit proof returned commitment is not active error");
                            let status = self.get_commitment_status().await?;
                            if let Some(status) = status {
                                tracing::info!(target: "chain-listener", "Current commitment status: {status}");
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

        Ok((
            U256::from_str_radix(&timestamp, 16)?,
            U256::from_str_radix(&block_number, 16)?,
        ))
    }
    async fn poll_deal_statuses(&mut self) -> eyre::Result<()> {
        if self.active_deals.is_empty() {
            return Ok(());
        }

        let statuses = retry(ExponentialBackoff::default(), || async {
            let s = self.chain_connector.get_deal_statuses(self.active_deals.keys()).await.map_err(|err| {
                tracing::error!(target: "chain-listener", "Failed to poll deal statuses: {err}");
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
                    DealStatus::InsufficientFunds | DealStatus::Ended => {
                        tracing::info!(target: "chain-listener", "Deal {deal_id} status: {status}; Exiting...");
                        self.exit_deal(&deal_id, cu_id).await?;
                        tracing::info!(target: "chain-listener", "Exited deal {deal_id} successfully");
                    }
                    DealStatus::Active
                    | DealStatus::NotEnoughWorkers
                    | DealStatus::SmallBalance => {}
                },
                Err(err) => {
                    tracing::error!(target: "chain-listener", "Failed to get deal status for {deal_id}: {err}");
                }
            }
        }

        Ok(())
    }
    async fn exit_deal(&mut self, deal_id: &DealId, cu_id: CUID) -> eyre::Result<()> {
        let mut backoff = ExponentialBackoff::default();
        backoff.max_elapsed_time = Some(Duration::from_secs(3));

        retry(backoff, || async {
            self.chain_connector.exit_deal(&cu_id).await.map_err(|err| {
                tracing::error!(target: "chain-listener", "Failed to exit deal {deal_id}: {err}");
                eyre!("Failed to exit deal {deal_id}: {err}; Retrying...")
            })?;
            Ok(())
        })
        .await?;

        self.active_deals.remove(deal_id);
        Ok(())
    }
}
