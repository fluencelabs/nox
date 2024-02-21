use crate::event::cc_activated::CommitmentActivated;
use crate::event::{
    CommitmentActivatedData, UnitActivated, UnitActivatedData, UnitDeactivated, UnitDeactivatedData,
};
use ccp_rpc_client::{CCPRpcHttpClient, OrHex};
use ccp_shared::proof::{CCProof, CCProofId, ProofIdx};
use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash};
use chain_connector::{ChainConnector, ConnectorError};
use chain_data::{next_opt, parse_chain_data, parse_log, peer_id_to_hex, ChainData, Log};
use chain_types::{
    CommitmentId, CommitmentStatus, ComputeUnit, COMMITMENT_IS_NOT_ACTIVE, TOO_MANY_PROOFS,
};
use core_manager::manager::{CoreManager, CoreManagerFunctions};
use core_manager::types::{AcquireRequest, WorkType};
use core_manager::CUID;
use cpu_utils::PhysicalCoreId;
use ethabi::ethereum_types::U256;
use ethabi::Token;
use jsonrpsee::core::client::{Client as WsClient, Subscription, SubscriptionClientT};
use jsonrpsee::core::{client, JsonValue};
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::WsClientBuilder;
use libp2p_identity::PeerId;
use serde_json::{json, Value};
use server_config::ChainConfig;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

pub struct ChainListener {
    chain_connector: Arc<ChainConnector>,
    ws_client: WsClient,
    ccp_client: CCPRpcHttpClient,
    config: ChainConfig,
    _cc_events_dir: PathBuf,
    host_id: PeerId,
    difficulty: Difficulty,
    init_timestamp: U256,
    global_nonce: GlobalNonce,
    current_epoch: U256,
    epoch_duration: U256,
    current_commitment: Option<CommitmentId>,
    active_compute_units: HashSet<CUID>,
    pending_compute_units: HashSet<ComputeUnit>,
    core_manager: Arc<CoreManager>,
}

async fn poll_subscription(
    s: &mut Option<Subscription<Log>>,
) -> Option<Result<Log, client::Error>> {
    match s {
        Some(ref mut s) => s.next().await,
        None => None,
    }
}

impl ChainListener {
    pub async fn new(
        config: ChainConfig,
        cc_events_dir: PathBuf,
        host_id: PeerId,
        chain_connector: Arc<ChainConnector>,
        core_manager: Arc<CoreManager>,
    ) -> eyre::Result<Self> {
        let init_params = chain_connector.get_cc_init_params().await?;
        let ws_client = WsClientBuilder::default()
            .build(&config.ws_endpoint)
            .await?;

        // We will use the first physical core for utility tasks
        let utility_core = core_manager
            .get_system_cpu_assignment()
            .physical_core_ids
            .first()
            .cloned()
            .ok_or(eyre::eyre!("No utility core id"))?;

        let ccp_client = CCPRpcHttpClient::new(config.ccp_endpoint.clone(), utility_core).await?;
        Ok(Self {
            chain_connector,
            ws_client,
            ccp_client,
            config,
            host_id,
            difficulty: init_params.difficulty,
            init_timestamp: init_params.init_timestamp,
            global_nonce: init_params.global_nonce,
            current_epoch: init_params.current_epoch,
            epoch_duration: init_params.epoch_duration,
            current_commitment: None,
            active_compute_units: HashSet::new(),
            pending_compute_units: HashSet::new(),
            core_manager,
            _cc_events_dir: cc_events_dir,
        })
    }

    pub fn start(mut self) -> JoinHandle<()> {
        let result = tokio::task::Builder::new()
            .name("ChainListener")
            .spawn(async move {
                let setup: eyre::Result<()> = try {
                    self.startup().await?;
                };
                if let Err(err) = setup {
                    log::error!("ChainListener startup error: {err}");
                    panic!("ChainListener startup error: {err}");
                }

                let mut heads = self.subscribe_new_heads().await.expect("Could not subscribe to new heads");
                let mut cc_events = self.subscribe_cc_activated().await.expect("Could not subscribe to cc events");

                let mut unit_activated: Option<Subscription<Log>>= None;
                let mut unit_deactivated: Option<Subscription<Log>> = None;

                let mut timer = IntervalStream::new(interval(self.config.timer_resolution));

                loop {
                    tokio::select! {
                        Some(header) = heads.next() => {
                            if let Err(err) = self.process_new_header(header).await {
                               log::error!("newHeads event processing error: {err}");
                            }
                        },
                        Some(cc) = cc_events.next() => {
                            match self.process_commitment_activated(cc).await {
                                Err(err) => log::error!("CommitmentActivated event processing error: {err}"),
                                Ok((activated, deactivated)) => {
                                    unit_activated = Some(activated);
                                    unit_deactivated = Some(deactivated);
                                }
                            }
                        },
                        Some(event) = poll_subscription(&mut unit_activated) => {
                           if let Err(err) = self.process_unit_activated(event).await {
                               log::error!("UnitActivated event processing error: {err}");
                           }
                        },
                         Some(event) = poll_subscription(&mut unit_deactivated) => {
                            if let Err(err) = self.process_unit_deactivated(event).await {
                                 log::error!("UnitDeactivated event processing error: {err}");
                            }
                        },
                        _ = timer.next() => {
                            if let Err(err) = self.submit_mocked_proofs().await {
                                log::error!("Failed to submit mocked proofs: {err}");
                            }
                            // self.poll_proofs().await?;
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

    /// Returns active, pending and deals compute units
    async fn get_compute_units(&self) -> eyre::Result<(Vec<CUID>, Vec<ComputeUnit>, Vec<CUID>)> {
        let mut units = self.chain_connector.get_compute_units().await?;
        let in_deals = units
            .extract_if(|unit| unit.deal.is_some())
            .map(|cu| cu.id)
            .collect();
        let (active, pending): (Vec<ComputeUnit>, Vec<ComputeUnit>) = units
            .into_iter()
            .partition(|unit| unit.start_epoch <= self.current_epoch);

        let active = active.into_iter().map(|unit| unit.id).collect();

        Ok((active, pending, in_deals))
    }
    async fn startup(&mut self) -> eyre::Result<()> {
        let (active, pending, in_deals) = self.get_compute_units().await?;
        self.core_manager.release(in_deals);
        self.current_commitment = self.chain_connector.get_current_commitment_id().await?;

        if let Some(status) = self.get_commitment_status().await? {
            match status {
                CommitmentStatus::Active => {
                    self.active_compute_units.extend(active);
                    self.pending_compute_units.extend(pending);
                    self.update_commitment().await?;
                }
                CommitmentStatus::WaitDelegation => {}
                CommitmentStatus::WaitStart => {}
                CommitmentStatus::Inactive
                | CommitmentStatus::Failed
                | CommitmentStatus::Removed => {
                    self.ccp_client.on_no_active_commitment().await?;
                }
            }
        }

        Ok(())
    }

    async fn subscribe_new_heads(&self) -> eyre::Result<Subscription<JsonValue>> {
        let subs = self
            .ws_client
            .subscribe("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    async fn subscribe_cc_activated(&self) -> eyre::Result<Subscription<Log>> {
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
            .await?;

        Ok(subs)
    }

    async fn subscribe_unit_activated(
        &self,
        commitment_id: &CommitmentId,
    ) -> eyre::Result<Subscription<Log>> {
        let params = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![UnitActivatedData::topic(), hex::encode(&commitment_id.0)]})
        ];
        let subs = self
            .ws_client
            .subscribe("eth_subscribe", params, "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    async fn subscribe_unit_deactivated(
        &self,
        commitment_id: &CommitmentId,
    ) -> eyre::Result<Subscription<Log>> {
        let params = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![UnitDeactivatedData::topic(), hex::encode(&commitment_id.0)]})
        ];
        let subs = self
            .ws_client
            .subscribe("eth_subscribe", params, "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    async fn process_new_header(
        &mut self,
        header: Result<Value, client::Error>,
    ) -> eyre::Result<()> {
        let timestamp = header?
            .as_object()
            .and_then(|o| o.get("timestamp"))
            .and_then(|v| v.as_str())
            .ok_or(eyre::eyre!("newHeads: no timestamp"))?
            .to_string();

        let mut tokens = parse_chain_data(&timestamp, &[ethabi::ParamType::Uint(256)])?.into_iter();
        let block_timestamp = next_opt(&mut tokens, "timestamp", Token::into_uint)?;

        // `epoch_number = 1 + (block_timestamp - init_timestamp) / epoch_duration`
        let epoch_number =
            (block_timestamp - self.init_timestamp) / self.epoch_duration + U256::from(1);
        let epoch_changed = epoch_number > self.current_epoch;

        if epoch_changed {
            self.current_epoch = epoch_number;
            // nonce changes every epoch
            self.global_nonce = self.chain_connector.get_global_nonce().await?;

            if let Some(status) = self.get_commitment_status().await? {
                match status {
                    CommitmentStatus::Active => {
                        self.activate_pending_units().await?;
                    }
                    CommitmentStatus::Inactive
                    | CommitmentStatus::Failed
                    | CommitmentStatus::Removed => {
                        self.stop_commitment().await?;
                    }
                    CommitmentStatus::WaitDelegation => {}
                    CommitmentStatus::WaitStart => {}
                }
            }
        }

        Ok(())
    }

    async fn process_commitment_activated(
        &mut self,
        event: Result<Log, client::Error>,
    ) -> eyre::Result<(Subscription<Log>, Subscription<Log>)> {
        let cc_event = parse_log::<CommitmentActivatedData, CommitmentActivated>(event?)?;
        let unit_ids = cc_event.info.unit_ids;
        let unit_activated = self
            .subscribe_unit_activated(&cc_event.info.commitment_id)
            .await?;
        let unit_deactivated = self
            .subscribe_unit_deactivated(&cc_event.info.commitment_id)
            .await?;

        if cc_event.info.start_epoch >= self.current_epoch {
            self.active_compute_units = unit_ids.into_iter().collect();
            self.update_commitment().await?;
        } else {
            self.pending_compute_units = unit_ids
                .into_iter()
                .map(|id| ComputeUnit::new(id, cc_event.info.start_epoch))
                .collect();
        }
        Ok((unit_activated, unit_deactivated))
    }

    async fn process_unit_activated(
        &mut self,
        event: Result<Log, client::Error>,
    ) -> eyre::Result<()> {
        let unit_event = parse_log::<UnitActivatedData, UnitActivated>(event?)?;
        if self.current_epoch >= unit_event.info.start_epoch {
            self.active_compute_units.insert(unit_event.info.unit_id);
            self.update_commitment().await?;
        } else {
            // Will be activated on the next epoch
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
        self.active_compute_units.remove(&unit_event.info.unit_id);
        self.pending_compute_units
            .retain(|cu| cu.id == unit_event.info.unit_id);
        self.update_commitment().await?;
        self.acquire_deal_core(unit_event.info.unit_id)?;
        Ok(())
    }

    async fn update_commitment(&self) -> eyre::Result<()> {
        let cores = self.acquire_capacity_cores()?;
        self.ccp_client
            .on_active_commitment(self.global_nonce, self.difficulty, cores)
            .await?;
        Ok(())
    }

    fn acquire_capacity_cores(&self) -> eyre::Result<HashMap<PhysicalCoreId, OrHex<CUID>>> {
        let cores = self.core_manager.acquire_worker_core(AcquireRequest::new(
            self.active_compute_units.clone().into_iter().collect(),
            WorkType::CapacityCommitment,
        ))?;

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

    async fn stop_commitment(&mut self) -> eyre::Result<()> {
        self.active_compute_units.clear();
        self.pending_compute_units.clear();
        self.current_commitment = None;
        self.ccp_client.on_no_active_commitment().await?;
        Ok(())
    }

    async fn activate_pending_units(&mut self) -> eyre::Result<()> {
        let to_activate = self
            .pending_compute_units
            .extract_if(|unit| unit.start_epoch <= self.current_epoch)
            .map(|cu| cu.id);

        self.active_compute_units.extend(to_activate);
        self.update_commitment().await?;
        Ok(())
    }

    /// Submit Mocked Proofs for all active compute units.
    /// Mocked Proof has result_hash == difficulty and random local_nonce
    async fn submit_mocked_proofs(&mut self) -> eyre::Result<()> {
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

    async fn submit_proof(&mut self, proof: CCProof) -> eyre::Result<()> {
        match self.chain_connector.submit_proof(proof).await {
            Ok(_) => Ok(()),
            Err(err) => {
                match err {
                    ConnectorError::RpcCallError { ref data, .. } => {
                        if data.contains(TOO_MANY_PROOFS) {
                            // we stop unit until the next epoch if "TooManyProofs" error received
                            self.active_compute_units.remove(&proof.cu_id);
                            self.pending_compute_units
                                .insert(ComputeUnit::new(proof.cu_id, self.current_epoch + 1));
                            self.update_commitment().await?;
                            Ok(())
                        } else if data.contains(COMMITMENT_IS_NOT_ACTIVE) {
                            self.stop_commitment().await?;
                            Ok(())
                        } else {
                            log::error!("Failed to submit proof: {err}");
                            Err(err.into())
                        }
                    }
                    _ => {
                        log::error!("Failed to submit proof: {err}");
                        Err(err.into())
                    }
                }
            }
        }
    }
}
