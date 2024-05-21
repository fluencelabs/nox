use alloy_primitives::hex::ToHexExt;
use alloy_primitives::{FixedBytes, Uint, U256};
use alloy_sol_types::sol_data::Array;
use alloy_sol_types::{SolCall, SolType};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use ccp_shared::proof::CCProof;
use ccp_shared::types::{Difficulty, GlobalNonce, CUID};
use clarity::{Transaction, Uint256};
use eyre::eyre;
use futures::FutureExt;
use jsonrpsee::core::async_trait;
use jsonrpsee::core::client::{BatchResponse, ClientT};
use jsonrpsee::core::params::{ArrayParams, BatchRequestBuilder};
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde::{Deserialize, Serialize};
use serde_json::Value as JValue;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::ConnectorError::{InvalidU256, ResponseParseError};
use crate::Deal::CIDV1;
use crate::{CCStatus, Capacity, CommitmentId, Core, Deal, Offer};
use chain_data::peer_id_to_bytes;
use fluence_libp2p::PeerId;
use hex_utils::decode_hex;
use particle_args::{Args, JError};
use particle_builtins::{wrap, CustomService};
use particle_execution::{ParticleParams, ServiceFunction};
use server_config::ChainConfig;
use types::peer_scope::WorkerId;
use types::DealId;

use crate::error::{process_response, ConnectorError};
use crate::Offer::{ComputePeer, ComputeUnit};

#[async_trait]
pub trait ChainConnector: Send + Sync {
    async fn get_current_commitment_id(&self) -> Result<Option<CommitmentId>, ConnectorError>;

    async fn get_cc_init_params(&self) -> eyre::Result<CCInitParams>; //TODO: make error type

    async fn get_compute_units(&self) -> Result<Vec<ComputeUnit>, ConnectorError>;

    async fn get_commitment_status(
        &self,
        commitment_id: CommitmentId,
    ) -> Result<CCStatus, ConnectorError>;

    async fn get_global_nonce(&self) -> Result<GlobalNonce, ConnectorError>;

    async fn submit_proof(&self, proof: CCProof) -> Result<String, ConnectorError>;

    async fn get_deal_statuses(
        &self,
        deal_ids: Vec<DealId>,
    ) -> Result<Vec<Result<Deal::Status, ConnectorError>>, ConnectorError>;

    async fn exit_deal(&self, cu_id: &CUID) -> Result<String, ConnectorError>;

    async fn get_tx_statuses(
        &self,
        tx_hashes: Vec<String>,
    ) -> Result<Vec<eyre::Result<bool>>, ConnectorError>;

    async fn get_tx_receipts(
        &self,
        tx_hashes: Vec<String>,
    ) -> Result<Vec<eyre::Result<TxReceiptResult>>, ConnectorError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DealInfo {
    pub deal_id: DealId,
    pub status: Deal::Status,
    pub unit_ids: Vec<Vec<u8>>,
    pub app_cid: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxReceiptResult {
    block_number: String,
    status: String,
    transaction_hash: String,
}

pub struct HttpChainConnector {
    client: Arc<jsonrpsee::http_client::HttpClient>,
    config: ChainConfig,
    tx_nonce_mutex: Arc<Mutex<()>>,
    host_id: PeerId,
}

pub struct CCInitParams {
    pub difficulty: Difficulty,
    pub init_timestamp: U256,
    pub global_nonce: GlobalNonce,
    pub current_epoch: U256,
    pub epoch_duration: U256,
    pub min_proofs_per_epoch: U256,
    pub max_proofs_per_epoch: U256,
}

impl HttpChainConnector {
    pub fn new(
        config: ChainConfig,
        host_id: PeerId,
    ) -> eyre::Result<(Arc<Self>, HashMap<String, CustomService>)> {
        tracing::info!(target: "chain-connector","Connecting to chain via {}", config.http_endpoint);

        let connector = Arc::new(Self {
            client: Arc::new(HttpClientBuilder::default().build(&config.http_endpoint)?),
            config,
            tx_nonce_mutex: Arc::new(Default::default()),
            host_id,
        });

        let builtins = Self::make_connector_builtins(connector.clone());
        Ok((connector, builtins))
    }

    fn make_connector_builtins(connector: Arc<Self>) -> HashMap<String, CustomService> {
        let mut builtins = HashMap::new();
        builtins.insert(
            "connector".to_string(),
            CustomService::new(
                vec![
                    ("send_tx", Self::make_send_tx_closure(connector.clone())),
                    ("get_deals", Self::make_get_deals_closure(connector.clone())),
                    (
                        "register_worker",
                        Self::make_register_worker_closure(connector.clone()),
                    ),
                    (
                        "get_tx_receipts",
                        Self::make_get_tx_receipts_closure(connector.clone()),
                    ),
                ],
                None,
            ),
        );
        builtins
    }

    fn make_send_tx_closure(connector: Arc<Self>) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |args, params| {
            let connector = connector.clone();
            async move { wrap(connector.send_tx_builtin(args, params).await) }.boxed()
        }))
    }

    fn make_get_deals_closure(connector: Arc<Self>) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |_, params| {
            let connector = connector.clone();
            async move { wrap(connector.get_deals_builtin(params).await) }.boxed()
        }))
    }

    fn make_register_worker_closure(connector: Arc<Self>) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |args, params| {
            let connector = connector.clone();
            async move { wrap(connector.register_worker_builtin(args, params).await) }.boxed()
        }))
    }

    fn make_get_tx_receipts_closure(connector: Arc<Self>) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |args, params| {
            let connector = connector.clone();
            async move { wrap(connector.get_tx_receipts_builtin(args, params).await) }.boxed()
        }))
    }

    async fn send_tx_builtin(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        if params.init_peer_id != self.host_id {
            return Err(JError::new("Only the root worker can send transactions"));
        }

        let mut args = args.function_args.into_iter();
        let data: Vec<u8> = Args::next("data", &mut args)?;
        let to: String = Args::next("to", &mut args)?;
        let tx_hash = self
            .send_tx(data, &to)
            .await
            .map_err(|err| JError::new(format!("Failed to send tx: {err}")))?;
        Ok(json!(tx_hash))
    }

    async fn get_deals_builtin(&self, params: ParticleParams) -> Result<JValue, JError> {
        if params.init_peer_id != self.host_id {
            return Err(JError::new("Only the root worker can call connector"));
        }

        let deals = self
            .get_deals()
            .await
            .map_err(|err| JError::new(format!("Failed to get deals: {err}")))?;
        Ok(json!(deals))
    }

    async fn register_worker_builtin(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        if params.init_peer_id != self.host_id {
            return Err(JError::new("Only the root worker can call connector"));
        }

        let mut args = args.function_args.into_iter();
        let deal_id: DealId = Args::next("deal_id", &mut args)?;
        let worker_id: WorkerId = Args::next("worker_id", &mut args)?;
        let cu_ids: Vec<CUID> = Args::next("cu_id", &mut args)?;

        if cu_ids.len() != 1 {
            return Err(JError::new("Only one cu_id is allowed"));
        }

        let tx_hash = self
            .register_worker(&deal_id, worker_id, cu_ids[0])
            .await
            .map_err(|err| JError::new(format!("Failed to register worker: {err}")))?;
        Ok(json!(tx_hash))
    }

    async fn get_tx_receipts_builtin(
        &self,
        args: Args,
        params: ParticleParams,
    ) -> Result<JValue, JError> {
        if params.init_peer_id != self.host_id {
            return Err(JError::new("Only the root worker can call connector"));
        }

        let mut args = args.function_args.into_iter();

        let tx_hashes: Vec<String> = Args::next("tx_hashes", &mut args)?;

        let receipts = self
            .get_tx_receipts(tx_hashes)
            .await
            .map_err(|err| JError::new(format!("Failed to get tx receipts: {err}")))?
            .into_iter()
            .map(|tx_receipt| match tx_receipt {
                Ok(receipt) => {
                    json!({
                        "success": json!(true),
                        "error":  json!([]),
                        "receipt": json!([json!({
                            "block_number": json!(receipt.block_number),
                            "tx_hash": json!(receipt.transaction_hash),
                            "status": json!(receipt.status),
                        })])
                    })
                }
                Err(err) => {
                    json!({
                        "success": json!(false),
                        "error":  json!(vec![err.to_string()]),
                        "receipt": json!([]),
                    })
                }
            })
            .collect::<Vec<_>>();

        Ok(json!(receipts))
    }

    async fn get_base_fee_per_gas(&self) -> Result<U256, ConnectorError> {
        if let Some(fee) = self.config.default_base_fee {
            return Ok(Uint::from(fee));
        }

        let block: Value = process_response(
            self.client
                .request("eth_getBlockByNumber", rpc_params!["pending", false])
                .await,
        )?;

        let fee = block
            .as_object()
            .and_then(|o| o.get("baseFeePerGas"))
            .and_then(Value::as_str)
            .ok_or(ResponseParseError(block.to_string()))?
            .to_string();

        let base_fee_per_gas =
            U256::from_str(&fee).map_err(|err| InvalidU256(fee, err.to_string()))?;

        Ok(base_fee_per_gas)
    }

    pub async fn get_app_cid(
        &self,
        deals: impl Iterator<Item = &DealId>,
    ) -> Result<Vec<String>, ConnectorError> {
        let data: String = Deal::appCIDCall {}.abi_encode().encode_hex();
        let mut batch = BatchRequestBuilder::new();
        for deal in deals {
            batch.insert(
                "eth_call",
                rpc_params![
                    json!({
                        "data": data,
                        "to": deal,
                    }),
                    "latest"
                ],
            )?;
        }
        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let mut cids = vec![];
        for result in resp.into_iter() {
            let cid = CIDV1::from_hex(&result?)?;
            let cid_bytes = [cid.prefixes.to_vec(), cid.hash.to_vec()].concat();
            let app_cid = libipld::Cid::read_bytes(cid_bytes.as_slice())?.to_string();

            cids.push(app_cid.to_string());
        }

        Ok(cids)
    }

    pub async fn get_deals(&self) -> eyre::Result<Vec<DealInfo>> {
        let units = self.get_compute_units().await?;
        let mut deals: BTreeMap<DealId, Vec<Vec<u8>>> = BTreeMap::new();

        units
            .iter()
            .filter(|unit| !unit.deal.is_zero())
            .for_each(|unit| {
                deals
                    .entry(unit.deal.to_string().into())
                    .or_insert_with(Vec::new)
                    .push(unit.id.to_vec());
            });

        if deals.is_empty() {
            return Ok(Vec::new());
        }

        let app_cids = self.get_app_cid(deals.keys()).await?;
        let statuses: Vec<Deal::Status> = self
            .get_deal_statuses(deals.keys().cloned().collect())
            .await?
            .into_iter()
            .collect::<Result<Vec<_>, ConnectorError>>()?;
        Ok(deals
            .into_iter()
            .zip(app_cids.into_iter().zip(statuses.into_iter()))
            .map(|((deal_id, unit_ids), (app_cid, status))| DealInfo {
                deal_id,
                status,
                unit_ids,
                app_cid,
            })
            .collect())
    }

    async fn get_tx_nonce(&self) -> Result<U256, ConnectorError> {
        let address = self.config.wallet_key.to_address().to_string();
        let resp: String = process_response(
            self.client
                .request("eth_getTransactionCount", rpc_params![address, "pending"])
                .await,
        )?;

        let nonce = U256::from_str(&resp).map_err(|err| InvalidU256(resp, err.to_string()))?;
        Ok(nonce)
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U256, ConnectorError> {
        if let Some(fee) = self.config.default_priority_fee {
            return Ok(Uint::from(fee));
        }

        let resp: String = process_response(
            self.client
                .request("eth_maxPriorityFeePerGas", rpc_params![])
                .await,
        )?;
        let max_priority_fee_per_gas =
            U256::from_str(&resp).map_err(|err| InvalidU256(resp, err.to_string()))?;
        Ok(max_priority_fee_per_gas)
    }

    async fn estimate_gas_limit(&self, data: &[u8], to: &str) -> Result<U256, ConnectorError> {
        let resp: String = process_response(
            self.client
                .request(
                    "eth_estimateGas",
                    rpc_params![json!({
                        "from": self.config.wallet_key.to_address().to_string(),
                        "to": to,
                        "data": format!("0x{}", hex::encode(data)),
                    })],
                )
                .await,
        )?;
        let limit = U256::from_str(&resp).map_err(|err| InvalidU256(resp, err.to_string()))?;
        Ok(limit)
    }

    pub async fn send_tx(&self, data: Vec<u8>, to: &str) -> Result<String, ConnectorError> {
        let base_fee = self.get_base_fee_per_gas().await?;
        tracing::info!(target: "chain-connector", "Estimating gas for tx from {} to {} data {}", self.config.wallet_key.to_address(), to, hex::encode(&data));
        let gas_limit = self.estimate_gas_limit(&data, to).await?;
        let max_priority_fee_per_gas = self.max_priority_fee_per_gas().await?;
        // (base fee + priority fee).
        let max_fee_per_gas = base_fee + max_priority_fee_per_gas;

        // We use this lock no ensure that we don't send two transactions with the same nonce
        let _lock = self.tx_nonce_mutex.lock().await;
        let nonce = self.get_tx_nonce().await?;

        // Create a new transaction
        let tx = Transaction::Eip1559 {
            chain_id: self.config.network_id.into(),
            nonce: Uint256::from_le_bytes(&nonce.to_le_bytes_vec()),
            max_priority_fee_per_gas: Uint256::from_le_bytes(
                &max_priority_fee_per_gas.to_le_bytes_vec(),
            ),
            gas_limit: Uint256::from_le_bytes(&gas_limit.to_le_bytes_vec()),
            to: to.parse()?,
            value: 0u32.into(),
            data,
            signature: None, // Not signed. Yet.
            max_fee_per_gas: Uint256::from_le_bytes(&max_fee_per_gas.to_le_bytes_vec()),
            access_list: vec![],
        };

        let tx = tx
            .sign(&self.config.wallet_key, Some(self.config.network_id))
            .to_bytes();
        let tx = hex::encode(tx);

        tracing::info!(target: "chain-connector",
            "Sending tx to {to} from {} signed {tx}",
            self.config.wallet_key.to_address()
        );

        let resp: String = process_response(
            self.client
                .request("eth_sendRawTransaction", rpc_params![format!("0x{}", tx)])
                .await,
        )?;
        Ok(resp)
    }

    pub async fn register_worker(
        &self,
        deal_id: &DealId,
        worker_id: WorkerId,
        cu_id: CUID,
    ) -> Result<String, ConnectorError> {
        let data = Deal::setWorkerCall {
            computeUnitId: cu_id.as_ref().into(),
            workerId: peer_id_to_bytes(worker_id.into()).into(),
        }
        .abi_encode();

        self.send_tx(data, &deal_id.as_str()).await
    }

    fn difficulty_params(&self) -> ArrayParams {
        let data: String = Core::difficultyCall {}.abi_encode().encode_hex();

        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }

    fn init_timestamp_params(&self) -> ArrayParams {
        let data: String = Core::initTimestampCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }
    fn global_nonce_params(&self) -> ArrayParams {
        let data: String = Capacity::getGlobalNonceCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.cc_contract_address}),
            "latest"
        ]
    }
    fn current_epoch_params(&self) -> ArrayParams {
        let data: String = Core::currentEpochCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }
    fn epoch_duration_params(&self) -> ArrayParams {
        let data: String = Core::epochDurationCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }

    fn min_proofs_per_epoch_params(&self) -> ArrayParams {
        let data: String = Core::minProofsPerEpochCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }

    fn max_proofs_per_epoch_params(&self) -> ArrayParams {
        let data: String = Core::maxProofsPerEpochCall {}.abi_encode().encode_hex();
        rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ]
    }
}

#[async_trait]
impl ChainConnector for HttpChainConnector {
    async fn get_current_commitment_id(&self) -> Result<Option<CommitmentId>, ConnectorError> {
        let peer_id = peer_id_to_bytes(self.host_id);
        let data: String = Offer::getComputePeerCall {
            peerId: peer_id.into(),
        }
        .abi_encode()
        .encode_hex();
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.market_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        let compute_peer = <ComputePeer as SolType>::abi_decode(&decode_hex(&resp)?, true)?;
        Ok(CommitmentId::new(compute_peer.commitmentId.0))
    }

    async fn get_cc_init_params(&self) -> eyre::Result<CCInitParams> {
        let mut batch = BatchRequestBuilder::new();

        batch.insert("eth_call", self.difficulty_params())?;
        batch.insert("eth_call", self.init_timestamp_params())?;
        batch.insert("eth_call", self.global_nonce_params())?;
        batch.insert("eth_call", self.current_epoch_params())?;
        batch.insert("eth_call", self.epoch_duration_params())?;
        batch.insert("eth_call", self.min_proofs_per_epoch_params())?;
        batch.insert("eth_call", self.max_proofs_per_epoch_params())?;

        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let mut results = resp
            .into_ok()
            .map_err(|err| eyre!("Some request failed in a batch {err:?}"))?;

        let difficulty: FixedBytes<32> =
            FixedBytes::from_str(&results.next().ok_or(eyre!("No response for difficulty"))?)?;
        let init_timestamp = U256::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for init_timestamp"))?,
        )?;

        let global_nonce = FixedBytes::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for global_nonce"))?,
        )?;

        let current_epoch = U256::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for current_epoch"))?,
        )?;

        let epoch_duration = U256::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for epoch_duration"))?,
        )?;

        let min_proofs_per_epoch = U256::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for min_proofs_per_epoch"))?,
        )?;

        let max_proofs_per_epoch = U256::from_str(
            &results
                .next()
                .ok_or(eyre!("No response for max_proofs_per_epoch"))?,
        )?;

        Ok(CCInitParams {
            difficulty: Difficulty::new(difficulty.0),
            init_timestamp,
            global_nonce: GlobalNonce::new(global_nonce.0),
            current_epoch,
            epoch_duration,
            min_proofs_per_epoch,
            max_proofs_per_epoch,
        })
    }

    async fn get_compute_units(&self) -> Result<Vec<ComputeUnit>, ConnectorError> {
        let data: String = Offer::getComputeUnitsCall {
            peerId: peer_id_to_bytes(self.host_id).into(),
        }
        .abi_encode()
        .encode_hex();

        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.market_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        let bytes = decode_hex(&resp)?;
        let compute_units = <Array<ComputeUnit> as SolType>::abi_decode(&bytes, true)?;

        Ok(compute_units)
    }

    async fn get_commitment_status(
        &self,
        commitment_id: CommitmentId,
    ) -> Result<CCStatus, ConnectorError> {
        let data: String = Capacity::getStatusCall {
            commitmentId: commitment_id.0.into(),
        }
        .abi_encode()
        .encode_hex();

        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.cc_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        Ok(<CCStatus as SolType>::abi_decode(
            &decode_hex(&resp)?,
            true,
        )?)
    }

    async fn get_global_nonce(&self) -> Result<GlobalNonce, ConnectorError> {
        let resp: String = process_response(
            self.client
                .request("eth_call", self.global_nonce_params())
                .await,
        )?;

        let bytes: FixedBytes<32> = FixedBytes::from_str(&resp)?;
        Ok(GlobalNonce::new(bytes.0))
    }

    async fn submit_proof(&self, proof: CCProof) -> Result<String, ConnectorError> {
        let data = Capacity::submitProofCall {
            unitId: proof.cu_id.as_ref().into(),
            localUnitNonce: proof.local_nonce.as_ref().into(),
            resultHash: proof.result_hash.as_ref().into(),
        }
        .abi_encode();

        self.send_tx(data, &self.config.cc_contract_address).await
    }

    async fn get_deal_statuses(
        &self,
        deal_ids: Vec<DealId>,
    ) -> Result<Vec<Result<Deal::Status, ConnectorError>>, ConnectorError> {
        let mut batch = BatchRequestBuilder::new();
        for deal_id in deal_ids {
            let data: String = Deal::getStatusCall {}.abi_encode().encode_hex();
            batch.insert(
                "eth_call",
                rpc_params![
                    json!({
                        "data": data,
                        "to": deal_id.to_address(),
                    }),
                    "latest"
                ],
            )?;
        }

        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let mut statuses = vec![];

        for status in resp.into_iter() {
            let status = status
                .map(|r| {
                    decode_hex(&r)
                        .map(|bytes| {
                            <Deal::Status as SolType>::abi_decode(&bytes, true)
                                .map_err(ConnectorError::ParseChainDataFailedAlloy)
                        })
                        .map_err(ConnectorError::DecodeHex)
                })
                .map_err(|e| ConnectorError::RpcError(e.to_owned().into()))??;
            statuses.push(status);
        }

        Ok(statuses)
    }
    async fn exit_deal(&self, cu_id: &CUID) -> Result<String, ConnectorError> {
        let data = Offer::returnComputeUnitFromDealCall {
            unitId: cu_id.as_ref().into(),
        }
        .abi_encode();

        self.send_tx(data, &self.config.market_contract_address)
            .await
    }

    async fn get_tx_statuses(
        &self,
        tx_hashes: Vec<String>,
    ) -> Result<Vec<eyre::Result<bool>>, ConnectorError> {
        let mut statuses = vec![];

        for receipt in self.get_tx_receipts(tx_hashes).await? {
            let status = receipt.map(|receipt| receipt.status == "0x1");
            statuses.push(status);
        }

        Ok(statuses)
    }

    async fn get_tx_receipts(
        &self,
        tx_hashes: Vec<String>,
    ) -> Result<Vec<eyre::Result<TxReceiptResult>>, ConnectorError> {
        let mut batch = BatchRequestBuilder::new();
        for tx_hash in tx_hashes {
            batch.insert("eth_getTransactionReceipt", rpc_params![tx_hash])?;
        }
        let resp: BatchResponse<Value> = self.client.batch_request(batch).await?;
        let mut receipts = vec![];
        for receipt in resp.into_iter() {
            let receipt = try {
                let receipt = receipt.map_err(|e| ConnectorError::RpcError(e.to_owned().into()))?;
                serde_json::from_value(receipt)?
            };
            receipts.push(receipt);
        }
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::uint;
    use alloy_primitives::U256;
    use std::assert_matches::assert_matches;
    use std::str::FromStr;
    use std::sync::Arc;

    use ccp_shared::proof::{CCProof, CCProofId, ProofIdx};
    use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash, CUID};
    use clarity::PrivateKey;
    use hex::FromHex;
    use mockito::Matcher;
    use serde::Deserialize;
    use serde_json::json;

    use chain_data::peer_id_from_hex;
    use fluence_libp2p::RandomPeerId;
    use hex_utils::decode_hex;

    use crate::Deal::Status::ACTIVE;
    use crate::{
        is_commitment_not_active, CCStatus, ChainConnector, CommitmentId, ConnectorError,
        HttpChainConnector,
    };

    fn get_connector(url: &str) -> Arc<HttpChainConnector> {
        let (connector, _) = HttpChainConnector::new(
            server_config::ChainConfig {
                http_endpoint: url.to_string(),
                cc_contract_address: "0x0E62f5cfA5189CA34E79CCB03829C064405790aD".to_string(),
                core_contract_address: "0x2f5224b7Cb8bd98d9Ef61c247F4741758E8E873d".to_string(),
                market_contract_address: "0x1dC1eB8fc8dBc35be6fE75ceba05C7D410a2e721".to_string(),
                network_id: 3525067388221321,
                wallet_key: PrivateKey::from_str(
                    "0x97a2456e78c4894c62eef6031972d1ca296ed40bf311ab54c231f13db59fc428",
                )
                .unwrap(),
                default_base_fee: None,
                default_priority_fee: None,
            },
            peer_id_from_hex("0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b")
                .unwrap(),
        )
        .unwrap();

        connector
    }

    #[tokio::test]
    async fn test_get_compute_units() {
        let expected_data = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000025d204dcc21f59c2a2098a277e48879207f614583e066654ad6736d36815ebb9e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000450e2f2a5bdb528895e9005f67e70fe213b9b822122e96fd85d2238cae55b6f900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();

        let units = get_connector(&url).get_compute_units().await.unwrap();

        mock.assert();
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].startEpoch, U256::from(0));
        assert!(units[0].deal.is_zero());
        assert_eq!(units[1].startEpoch, U256::from(0));
        assert!(units[1].deal.is_zero());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_none() {
        let expected_data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();
        let commitment_id = get_connector(&url)
            .get_current_commitment_id()
            .await
            .unwrap();

        mock.assert();
        assert!(commitment_id.is_none());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_some() {
        let expected_data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();
        let commitment_id = get_connector(&url)
            .get_current_commitment_id()
            .await
            .unwrap();

        mock.assert();
        assert!(commitment_id.is_some());
        assert_eq!(
            hex::encode(commitment_id.unwrap().0),
            "aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5"
        );
    }

    #[tokio::test]
    async fn get_commitment_status() {
        let commitment_id = "0xa98dc43600773b162bcdb8175eadc037412cd7ad83555fafa507702011a53992";

        let expected_data = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({
                "method": "eth_call",
            })))
            .with_body(expected_response)
            .create();
        let commitment_id = CommitmentId(decode_hex(commitment_id).unwrap().try_into().unwrap());
        let status = get_connector(&url)
            .get_commitment_status(commitment_id)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(status, CCStatus::Active);
    }

    #[tokio::test]
    async fn test_batch_init_request() {
        let expected_response = r#"[
          {
            "jsonrpc": "2.0",
            "result": "0x76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec",
            "id": 0
          },
          {
            "jsonrpc": "2.0",
            "result": "0x0000000000000000000000000000000000000000000000000000000065ca5a01",
            "id": 1
          },
          {
            "jsonrpc": "2.0",
            "result": "0x0000000000000000000000000000000000000000000000000000000000000005",
            "id": 2
          },
          {
            "jsonrpc": "2.0",
            "result": "0x00000000000000000000000000000000000000000000000000000000000016be",
            "id": 3
          },
          {
            "jsonrpc": "2.0",
            "result": "0x000000000000000000000000000000000000000000000000000000000000000f",
            "id": 4
          },
          {
            "jsonrpc": "2.0",
            "result": "0x5",
            "id": 5
          },
          {
            "jsonrpc": "2.0",
            "result": "0x8",
            "id": 6
          }
        ]"#;

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();

        let init_params = get_connector(&url).get_cc_init_params().await.unwrap();

        mock.assert();
        assert_eq!(
            init_params.difficulty,
            <Difficulty>::from_hex(
                "76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec"
            )
            .unwrap()
        );
        assert_eq!(init_params.init_timestamp, uint!(1707760129_U256));
        assert_eq!(
            init_params.global_nonce,
            <GlobalNonce>::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000005"
            )
            .unwrap()
        );
        assert_eq!(
            init_params.current_epoch,
            U256::from(0x00000000000000000000000000000000000000000000000000000000000016be)
        );
        assert_eq!(
            init_params.epoch_duration,
            U256::from(0x000000000000000000000000000000000000000000000000000000000000000f)
        );
        assert_eq!(init_params.min_proofs_per_epoch, U256::from(5));
        assert_eq!(init_params.max_proofs_per_epoch, U256::from(8));
    }

    #[tokio::test]
    async fn submit_proof_not_active() {
        let get_block_by_number = r#"{"jsonrpc":"2.0","id":0,"result":{"hash":"0xcbe8d90665392babc8098738ec78009193c99d3cc872a6657e306cfe8824bef9","parentHash":"0x15e767118a3e2d7545fee290b545faccd4a9eff849ac1057ce82cab7100c0c52","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0xa2","gasLimit":"0x1c9c380","gasUsed":"0x0","timestamp":"0x65d88f76","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","baseFeePerGas":"0x7","totalDifficulty":"0x0","uncles":[],"transactions":[],"size":"0x220"}}"#;
        let estimate_gas = r#"
        {
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": 3,
                "message": "execution reverted: ",
                "data": "0x0852c7200000000000000000000000000000000000000000000000000000000000000000"
            }
        }
        "#;

        let mut server = mockito::Server::new();
        let url = server.url();
        server
            .mock("POST", "/")
            // expect exactly 4 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                let method = body.get("method").expect("get method");
                let method = method.as_str().expect("as str").trim_matches(|c| c == '\"');

                match method {
                    "eth_getBlockByNumber" => get_block_by_number.into(),
                    "eth_estimateGas" => estimate_gas.into(),
                    method => format!("'{}' not supported", method).into(),
                }
            })
            .create();

        let proof = CCProof::new(
            CCProofId::new(
                GlobalNonce::new([0u8; 32]),
                Difficulty::new([0u8; 32]),
                ProofIdx::zero(),
            ),
            LocalNonce::new([0u8; 32]),
            CUID::new([0u8; 32]),
            ResultHash::from_slice([0u8; 32]),
        );
        let result = get_connector(&url).submit_proof(proof).await;

        assert!(result.is_err());

        assert_matches!(
            result.unwrap_err(),
            ConnectorError::RpcCallError {
                code: _,
                message: _,
                data,
            } if is_commitment_not_active(&data)
        );
    }

    #[tokio::test]
    async fn submit_proof() {
        let get_block_by_number = r#"{"jsonrpc":"2.0","id":0,"result":{"hash":"0xcbe8d90665392babc8098738ec78009193c99d3cc872a6657e306cfe8824bef9","parentHash":"0x15e767118a3e2d7545fee290b545faccd4a9eff849ac1057ce82cab7100c0c52","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0xa2","gasLimit":"0x1c9c380","gasUsed":"0x0","timestamp":"0x65d88f76","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","baseFeePerGas":"0x7","totalDifficulty":"0x0","uncles":[],"transactions":[],"size":"0x220"}}"#;
        let estimate_gas = r#"{"jsonrpc":"2.0","id": 1,"result": "0x5208"}"#;
        let max_priority_fee = r#"{"jsonrpc":"2.0","id": 2,"result": "0x5208"}"#;
        let nonce = r#"{"jsonrpc":"2.0","id":3,"result":"0x20"}"#;
        let send_tx_response = r#"
            {
                "jsonrpc": "2.0",
                "id": 4,
                "result": "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
            }
            "#;
        let mut server = mockito::Server::new();
        let url = server.url();
        server
            .mock("POST", "/")
            // expect exactly 4 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                let method = body.get("method").expect("get method");
                let method = method.as_str().expect("as str").trim_matches(|c| c == '\"');

                match method {
                    "eth_getBlockByNumber" => get_block_by_number.into(),
                    "eth_estimateGas" => estimate_gas.into(),
                    "eth_maxPriorityFeePerGas" => max_priority_fee.into(),
                    "eth_getTransactionCount" => nonce.into(),
                    "eth_sendRawTransaction" => send_tx_response.into(),
                    method => format!("'{}' not supported", method).into(),
                }
            })
            .create();

        let proof = CCProof::new(
            CCProofId::new(
                GlobalNonce::new([0u8; 32]),
                Difficulty::new([0u8; 32]),
                ProofIdx::zero(),
            ),
            LocalNonce::new([0u8; 32]),
            CUID::new([0u8; 32]),
            ResultHash::from_slice([0u8; 32]),
        );
        let result = get_connector(&url).submit_proof(proof).await.unwrap();

        assert_eq!(
            result,
            "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
        );
    }

    #[derive(Debug, Deserialize)]
    // for parsing
    #[allow(dead_code)]
    struct RpcRequest<P> {
        jsonrpc: String,
        id: u64,
        method: String,
        params: P,
    }

    #[derive(Debug, Deserialize)]
    // for parsing
    #[allow(dead_code)]
    struct EthCall {
        data: String,
        to: String,
    }

    #[tokio::test]
    async fn test_get_app_cid_empty() {
        // This response causes the `buffer overrun while deserializing` error
        let app_cid_response = r#"[{"jsonrpc":"2.0", "id":0, "result": "0x"}]"#;

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(app_cid_response)
            .create();

        let deals = vec![types::DealId::from(
            "0x0000000000000000000000000000000000000000",
        )];
        let result = get_connector(&url).get_app_cid(deals.iter()).await;
        assert_matches!(result, Err(ConnectorError::EmptyData(_)));

        mock.assert();
    }

    #[tokio::test]
    async fn test_get_deals() {
        let expected_deal_id = "5e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c";
        let expected_cuid = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5";
        let compute_units_response = "00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000001aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d50000000000000000000000005e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c00000000000000000000000000000000000000000000000000000000000fffbc";
        let compute_units_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{compute_units_response}\",\"id\":0}}");
        let expected_app_cid = "bafkreiekvwp2w7t7vw4jzjq4s4n4wc323c6dnexmy4axh6c7tiza5wxzm4";
        let app_cid_response = "0x01551220000000000000000000000000000000000000000000000000000000008aad9fab7e7fadb89ca61c971bcb0b7ad8bc3692ecc70173f85f9a320edaf967";
        let app_cid_response =
            format!("[{{\"jsonrpc\":\"2.0\",\"result\":\"{app_cid_response}\",\"id\":1}}]");
        let deal_status_response =
            "0x0000000000000000000000000000000000000000000000000000000000000001";
        let deal_status_response =
            format!("[{{\"jsonrpc\":\"2.0\",\"result\":\"{deal_status_response}\",\"id\":2}}]");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            .expect(3)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                match body {
                    serde_json::Value::Object(_) => {
                        let call: RpcRequest<(EthCall, String)> =
                            serde_json::from_value(body).expect("parse eth call request");
                        match call.params.0.data.as_ref() {
                            // compute units selector
                            "b6015c6e6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b" => {
                                compute_units_response.clone().into()

                            },
                            _ => {
                                panic!("unexpected call: {call:?}");
                            }
                        }
                    }
                    serde_json::Value::Array(_) => {
                        let calls: Vec<RpcRequest<(EthCall, String)>> =
                            serde_json::from_value(body).expect("parse eth call request");
                        match calls[0].params.0.data.as_ref() {
                            // app cid selector
                            "9bc66868" => app_cid_response.clone().into(),
                            // deal status selector
                            "4e69d560" => deal_status_response.clone().into(),
                            _ => {
                                panic!("unexpected call: {:?}", calls[0]);
                            }
                        }
                    }
                    x => {
                        panic!("unexpected body: {x:?}");
                    }
                }
            })
            .create();

        let deals = get_connector(&url).get_deals().await.unwrap();
        assert_eq!(deals.len(), 1, "there should be only one deal: {deals:?}");
        assert_eq!(deals[0].deal_id, expected_deal_id);
        assert_eq!(deals[0].status, ACTIVE);
        assert_eq!(
            deals[0].unit_ids.len(),
            1,
            "there should be only one unit id: {deals:?}"
        );
        assert_eq!(hex::encode(&deals[0].unit_ids[0]), expected_cuid);
        assert_eq!(deals[0].app_cid, expected_app_cid);
        mock.assert();
    }

    #[tokio::test]
    async fn test_get_deals_no_deals() {
        let compute_units_response = "0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002097ab88db9d8e48a1d55cd0205bd74721eb2ba70897d255f350062dec8fec7bb4000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002988e97756c550265b7a570b41bb4d8dbf056a1ca0435265ad175c5ecced2ef600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d4af56c3a07423c4b976f6d1a43a96f4843bdc5479b01bf3bd73f69c1062738e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e9ab8214c37d615708ef89559407e1a015a0f7fbdaf3cd23a1f4c5ed2ae1c8cd00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a717255e798920c9a9bbaae45492ff9020cdc8db3d8a44099015fbe950927368000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000df891a54a5479aa7ee3c019615ac55a18534aa81dcf3b2fbf7577d54cadf0c300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000fbbd2b72d216685d97c09c56ca6ba16e7d2e35ff72bb69c72584ad8d9365610200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ccff78d7a0c365cd3ba8842707a2f96f26ac7ea4dbd7aac04ae2f0958ef2252400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000db5fba69f5d2b2d04a5bf63b7abe1a2781470e77101e3afe6ec66794aec80c0a000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005b96d1d71cf1b77421645ae4031558195d9df60abd73ea716c3df67d3e7832da00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e5cf187d01e552b0bd38ea89b9013e7b98915891cafb44fc7cf6937223290bcf00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000fe00f5915737e75aa5c7799dcf326020ab499a65c9d10e1e4951be8371aee6f2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000c073f4fe89ebe1a354f052de5e371dc39e8d2b8f9ccbf0fc9e26e107217fe40000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008510f943f7080e58846bc5044afa0c91480dbeca0ff5dcbef7c522a43531cffc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007d3ea3803085980a5b4e3bd93c32d8dae3b8060db9aab800e0922b7b18d865fc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002be7bd75150701c7ef8521d322fbcbb228cf907224d80e433550a3034bbcbb8b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e1469c97990a40da4465cb4e749faa2905adfc4b1109f7dc8b66ef8b1ed0a0c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008d20e7f50737ea4236c6d92b150e569aa2fc482699cc0165722210b4bcffdc4c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000eae742035300aed3774579cab0c75416002438809f3cb3f62fb3c34dd0ab16790000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000029e9cc0f707e8e02578285b8e1d50c207fab5ec11b0b1e6f97834e1154abbe4b000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005be58fc7ae8eb4e577b1be2a911a11fd17b90c8b6754aa71859ad6285bcc3b00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008f7ba4e1f4bccd2aaf1f8791df5b73a6727b0720aba8fcf0afb6e1f07303c1cc0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000053f668a26f5978f4f252145d68d2f0a627116c197a342a2e42aa269f46dcccf000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000db7ddcd38d772f1b043572bab7d890ea65723cf4805ee963f46b7a9f81a454f20000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000018fa36dd54f48c4c4ae40ceefca274460d4c83026f48fdcc4e201a1a0423561400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a0b6e3c818a1ecb540a289c81d0835dc41755f091ba70f02e5134203c195b80900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000e77ef1585872eabfa78c01915cf0d43c3bd3bd63ae40565c62254a95bba901530000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000059810dcb510b17ca28693caa6d6164b6db28925290f345d8ef0ac8ef5751014f0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d6b992345e2ed6287ff2df0943d10f972ae7f63789d11df22f3fd9a4199f5c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005e58c2b5ba2b1a7d861de440a962fe0ffdcef1e240e3e50690f6f1f527bf7d66000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ca0be7865c187df9331bce42cd6ea3498d858a3d8b37fec18e9654ef320daa7000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003938f3a12b19ed989431771df29f64ef011eec4da79e2ec30d86f28dae35f10a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let compute_units_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{compute_units_response}\",\"id\":0}}");
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(compute_units_response)
            .create();

        let deals = get_connector(&url).get_deals().await;
        assert!(
            deals.is_ok(),
            "must not fail when no deals in compute units"
        );
        assert!(
            deals.unwrap().is_empty(),
            "no deals must be found in the compute units"
        );

        mock.assert();
    }

    #[tokio::test]
    async fn test_register_worker() {
        let get_block_by_number_response = r#"{"jsonrpc":"2.0","id":0,"result":{"hash":"0xcbe8d90665392babc8098738ec78009193c99d3cc872a6657e306cfe8824bef9","parentHash":"0x15e767118a3e2d7545fee290b545faccd4a9eff849ac1057ce82cab7100c0c52","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0xa2","gasLimit":"0x1c9c380","gasUsed":"0x0","timestamp":"0x65d88f76","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","baseFeePerGas":"0x7","totalDifficulty":"0x0","uncles":[],"transactions":[],"size":"0x220"}}"#;
        let estimate_gas_response = r#"{"jsonrpc":"2.0","id": 1,"result": "0x5208"}"#;
        let max_priority_fee_response = r#"{"jsonrpc":"2.0","id": 2,"result": "0x5208"}"#;
        let nonce_response = r#"{"jsonrpc":"2.0","id":3,"result":"0x20"}"#;
        let expected_tx_hash = "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05";
        let send_tx_response = r#"
            {
                "jsonrpc": "2.0",
                "id": 4,
                "result": "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
            }
            "#;

        let deal_id = "5e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c";
        let deal_id = types::DealId::from(deal_id);
        let worker_id = types::peer_scope::WorkerId::from(RandomPeerId::random());
        let cuid =
            CUID::from_hex("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
                .unwrap();

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            .expect(5)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: RpcRequest<Option<serde_json::Value>> =
                    serde_json::from_slice(body).expect("mock: parse request body");
                match body.method.as_ref() {
                    "eth_getBlockByNumber" => get_block_by_number_response.into(),
                    "eth_estimateGas" => estimate_gas_response.into(),
                    "eth_maxPriorityFeePerGas" => max_priority_fee_response.into(),
                    "eth_getTransactionCount" => nonce_response.into(),
                    "eth_sendRawTransaction" => send_tx_response.into(),
                    x => {
                        panic!("unknown method: {x}. Request {body:?}");
                    }
                }
            })
            .create();

        let result = get_connector(&url)
            .register_worker(&deal_id, worker_id, cuid)
            .await
            .unwrap();
        assert_eq!(result, expected_tx_hash);

        mock.assert();
    }

    #[tokio::test]
    async fn test_get_receipts() {
        let tx_hash =
            "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05".to_string();
        let receipt_response = r#"
            [{
                "id": 0,
                "jsonrpc": "2.0",
                "result": {
                    "blockNumber": "0x123",
                    "status": "0x1",
                    "transactionHash": "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
                }
            }]
        "#;
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: Vec<RpcRequest<serde_json::Value>> =
                    serde_json::from_slice(body).expect("mock: parse request body");
                match body[0].method.as_ref() {
                    "eth_getTransactionReceipt" => receipt_response.into(),
                    x => {
                        panic!("unknown method: {x}. Request {body:?}");
                    }
                }
            })
            .create();

        let mut result = get_connector(&url)
            .get_tx_receipts(vec![tx_hash.clone()])
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].is_ok(), "can't get receipt: {:?}", result[0]);

        let result = result.remove(0).unwrap();
        assert_eq!(result.transaction_hash, tx_hash);
        mock.assert();
    }
}
