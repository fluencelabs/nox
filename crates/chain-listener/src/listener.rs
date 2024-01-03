use crate::event::cc_activated::CCActivated;
use crate::event::CCActivatedData;
use chain_data::{parse_log, peer_id_to_hex, ChainData, Log};
use jsonrpsee::core::client::{Client, Subscription, SubscriptionClientT};
use jsonrpsee::core::JsonValue;
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::WsClientBuilder;
use libp2p_identity::PeerId;
use serde_json::json;
use server_config::ChainListenerConfig;
use std::path::PathBuf;
use tokio::task::JoinHandle;

pub struct ChainListener {
    config: ChainListenerConfig,
    cc_events_dir: PathBuf,
    host_id: PeerId,
}

impl ChainListener {
    pub fn new(config: ChainListenerConfig, cc_events_dir: PathBuf, host_id: PeerId) -> Self {
        Self {
            config,
            cc_events_dir,
            host_id,
        }
    }
    pub fn start(self) -> JoinHandle<()> {
        let result = tokio::task::Builder::new()
            .name("ChainListener")
            .spawn(async move {
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
                                        let cc_event = parse_log::<CCActivatedData, CCActivated>(event)?;
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
        client: &Client,
    ) -> eyre::Result<Subscription<JsonValue>> {
        let subs = client
            .subscribe("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    pub async fn subscribe_cc_activated(&self, client: &Client) -> eyre::Result<Subscription<Log>> {
        let topics = vec![CCActivatedData::topic(), peer_id_to_hex(self.host_id)];
        let params = rpc_params![
            "logs",
            json!({"address": self.config.cc_contract_address, "topics": topics})
        ];
        let subs = client
            .subscribe("eth_subscribe", params, "eth_unsubscribe")
            .await?;

        Ok(subs)
    }

    pub async fn save_cc_event(&self, event: CCActivated) -> eyre::Result<()> {
        let file_name = format!("{}.json", hex::encode(&event.info.commitment_id.0));
        let file_path = self.cc_events_dir.join(file_name);
        tokio::fs::write(file_path, serde_json::to_string(&event)?).await?;
        Ok(())
    }
}
