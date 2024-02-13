use crate::event::cc_activated::CommitmentActivated;
use crate::event::{CommitmentActivatedData, UnitActivatedData, UnitDeactivatedData};
use chain_connector::ChainConnector;
use chain_data::{parse_log, peer_id_to_hex, ChainData, Log};
use chain_types::{CommitmentId, CommitmentStatus, ComputeUnit};
use ethabi::ethereum_types::U256;
use jsonrpsee::core::client::{Client as WsClient, Subscription, SubscriptionClientT};
use jsonrpsee::core::JsonValue;
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::WsClientBuilder;
use libp2p_identity::PeerId;
use serde_json::json;
use server_config::ChainListenerConfig;
use std::path::PathBuf;
use tokio::task::{JoinHandle, JoinSet};

pub struct ChainListener {
    config: ChainListenerConfig,
    cc_events_dir: PathBuf,
    host_id: PeerId,
    difficulty: Vec<u8>,
    init_timestamp: U256,
    global_nonce: Vec<u8>,
    current_epoch: U256,
    epoch_duration: U256,
    current_commitment: Option<CommitmentId>,
    compute_units: Vec<ComputeUnit>,
}

impl ChainListener {
    pub async fn new(
        config: ChainListenerConfig,
        cc_events_dir: PathBuf,
        host_id: PeerId,
    ) -> eyre::Result<Self> {
        let init_params = ChainConnector::batch_init_request(&config).await?;
        let current_commitment = ChainConnector::get_current_commitment_id(
            &config.http_endpoint,
            &config.market_contract_address,
            host_id,
        )
        .await?;
        let compute_units =
            ChainConnector::get_compute_units(&config.http_endpoint, "", host_id).await?;
        Ok(Self {
            config,
            cc_events_dir,
            host_id,
            difficulty: init_params.difficulty,
            init_timestamp: init_params.init_timestamp,
            global_nonce: init_params.global_nonce,
            current_epoch: init_params.current_epoch,
            epoch_duration: init_params.epoch_duration,
            current_commitment,
            compute_units,
        })
    }

    pub fn start(self) -> JoinHandle<()> {
        let result = tokio::task::Builder::new()
            .name("ChainListener")
            .spawn(async move {
                let setup: eyre::Result<()> = try {
                    if let Some(commitment_id) = self.current_commitment.clone() {
                        let commitment = ChainConnector::get_commitment(&self.config.http_endpoint, &self.config.cc_contract_address, commitment_id).await?;
                        if commitment.status == CommitmentStatus::Active {
                            if commitment.start_epoch > self.current_epoch {
                                // TODO: acquire compute units
                            } else {
                                // TODO: Put in queue
                            }
                        } else {
                            // TODO: release compute units
                        }
                    }
                };

                let client = WsClientBuilder::default()
                    .build(&self.config.ws_endpoint)
                    .await
                    .unwrap_or_else(|_| panic!("Could not connect to chain {}", self.config.ws_endpoint));

                let mut _heads = self.subscribe_new_heads(&client).await.expect("Could not subscribe to new heads");
                let mut cc_events = self.subscribe_cc_activated(&client).await.expect("Could not subscribe to cc events");

                loop {
                    tokio::select! {
                        _ = _heads.next() => {},
                        Some(cc) = cc_events.next() => {
                            match cc {
                                Ok(event) => {
                                    let res: eyre::Result<()> = try {
                                        let cc_event = parse_log::<CommitmentActivatedData, CommitmentActivated>(event)?;
                                        let unit_events = self.subscribe_unit_events(&client, cc_event.info.commitment_id.clone()).await?;
                                        self.save_cc_event(cc_event).await?;
                                    };

                                   if let Err(err) = res {
                                       log::error!("CC event processing error: {err}");
                                   }
                                }
                                Err(err) => {
                                    log::error!("CC events subscription error: {err}");
                                }
                            };
                        }
                    }
                }
            })
            .expect("Could not spawn task");

        result
    }

    pub async fn subscribe_new_heads(
        &self,
        client: &WsClient,
    ) -> eyre::Result<Subscription<JsonValue>> {
        let subs = client
            .subscribe("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    pub async fn subscribe_cc_activated(
        &self,
        client: &WsClient,
    ) -> eyre::Result<Subscription<Log>> {
        let topics = vec![
            CommitmentActivatedData::topic(),
            peer_id_to_hex(self.host_id),
        ];
        let params = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics": topics})
        ];
        let subs = client
            .subscribe("eth_subscribe", params, "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    pub async fn subscribe_unit_events(
        &self,
        client: &WsClient,
        commitment_id: CommitmentId,
    ) -> eyre::Result<(Subscription<Log>, Subscription<Log>)> {
        let params_activated = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![UnitActivatedData::topic(), hex::encode(&commitment_id.0)]})
        ];
        let subs_activated = client
            .subscribe("eth_subscribe", params_activated, "eth_unsubscribe")
            .await?;
        let params_deactivated = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics":  vec![UnitDeactivatedData::topic(), hex::encode(&commitment_id.0)]})
        ];
        let subs_deactivated = client
            .subscribe("eth_subscribe", params_deactivated, "eth_unsubscribe")
            .await?;

        Ok((subs_activated, subs_deactivated))
    }

    pub async fn save_cc_event(&self, event: CommitmentActivated) -> eyre::Result<()> {
        let file_name = format!("{}.json", hex::encode(&event.info.commitment_id.0));
        let file_path = self.cc_events_dir.join(file_name);
        tokio::fs::write(file_path, serde_json::to_string(&event)?).await?;
        Ok(())
    }
}
