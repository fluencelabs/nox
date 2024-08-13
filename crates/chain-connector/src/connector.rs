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

use alloy_primitives::{FixedBytes, Uint, U256};
use alloy_sol_types::sol_data::Array;
use alloy_sol_types::{SolCall, SolType};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;

use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash, CUID};
use clarity::{Transaction, Uint256};
use eyre::eyre;
use jsonrpsee::core::async_trait;
use jsonrpsee::core::client::{BatchResponse, ClientT};
use jsonrpsee::core::params::{ArrayParams, BatchRequestBuilder};
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::builtins::make_connector_builtins;
use crate::error::process_response;
use crate::eth_call::EthCall;
use crate::types::*;
use crate::ConnectorError::{FieldNotFound, InvalidU256, ResponseParseError};
use crate::Deal::CIDV1;
use crate::Offer::{ComputePeer, ComputeUnit};
use crate::{CCStatus, Capacity, CommitmentId, Core, Deal, Offer, OnChainWorkerID};
use chain_data::{peer_id_to_bytes, BlockHeader};
use fluence_libp2p::PeerId;
use hex_utils::{decode_hex, encode_hex_0x};
use particle_builtins::CustomService;
use server_config::ChainConfig;
use types::peer_scope::WorkerId;
use types::DealId;

#[async_trait]
pub trait ChainConnector: Send + Sync {
    async fn get_current_commitment_id(&self) -> Result<Option<CommitmentId>>;

    async fn get_cc_init_params(&self) -> eyre::Result<CCInitParams>; //TODO: make error type

    async fn get_compute_units(&self) -> Result<Vec<ComputeUnit>>;

    async fn get_commitment_status(&self, commitment_id: CommitmentId) -> Result<CCStatus>;

    async fn get_global_nonce(&self) -> Result<GlobalNonce>;

    async fn submit_proofs(
        &self,
        unit_ids: Vec<CUID>,
        local_nonces: Vec<LocalNonce>,
        result_hashes: Vec<ResultHash>,
    ) -> Result<String>;

    async fn get_deal_statuses(&self, deal_ids: Vec<DealId>) -> Result<Vec<Result<Deal::Status>>>;

    async fn exit_deal(
        &self,
        deal_id: &DealId,
        on_chain_worker_id: OnChainWorkerID,
    ) -> Result<String>;

    async fn get_tx_statuses(&self, tx_hashes: Vec<String>) -> Result<Vec<Result<Option<bool>>>>;

    async fn get_tx_receipts(&self, tx_hashes: Vec<String>) -> Result<Vec<Result<TxStatus>>>;
}

pub struct HttpChainConnector {
    client: Arc<jsonrpsee::http_client::HttpClient>,
    config: ChainConfig,
    tx_nonce_mutex: Arc<Mutex<Option<U256>>>,
    pub(crate) host_id: PeerId,
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
            tx_nonce_mutex: Arc::new(Mutex::new(None)),
            host_id,
        });

        let builtins = make_connector_builtins(connector.clone());
        Ok((connector, builtins))
    }

    async fn get_base_fee_per_gas(&self) -> Result<U256> {
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

    pub async fn get_app_cid(&self, deals: impl Iterator<Item = &DealId>) -> Result<Vec<String>> {
        let data = Deal::appCIDCall {}.abi_encode();
        let mut batch = BatchRequestBuilder::new();
        for deal in deals {
            batch.insert(
                "eth_call",
                rpc_params![EthCall::to(&data, &deal.to_address()), "latest"],
            )?;
        }
        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let mut cids = vec![];
        for result in resp.into_iter() {
            let cid = CIDV1::from_hex(&result?)?;
            let app_cid = cid.to_ipld()?;
            cids.push(app_cid.to_string());
        }

        Ok(cids)
    }

    pub(crate) async fn get_deals(&self) -> eyre::Result<Vec<DealResult>> {
        let units = self.get_compute_units().await?;
        tracing::debug!(target: "chain-connector", "get_deals: Got {} compute units", units.len());
        let mut deals: BTreeMap<DealId, BTreeMap<OnChainWorkerId, Vec<CUID>>> = BTreeMap::new();

        units
            .iter()
            .filter(|unit| !unit.deal.is_zero())
            .for_each(|unit| {
                deals
                    .entry(unit.deal.to_string().into())
                    .or_default()
                    .entry(unit.onchainWorkerId.as_slice().into())
                    .or_default()
                    .push(CUID::new(unit.id.into()));
            });

        println!("Deals: {deals:?}");
        // For now, we forbid multiple workers for one deal on the peer!
        deals.retain(|deal_id, worker| {
            if worker.keys().len() > 1 {
                tracing::warn!(target: "chain-connector", "Deal {deal_id} has more then one worker for the peer which is forbiden at the moment");
               false
            } else {
               true
            }
        });

        if deals.is_empty() {
            return Ok(Vec::new());
        }

        tracing::debug!(target: "chain-connector", "get_deals: Got {} deals: {:?}", deals.len(), deals);
        let infos = self.get_deal_infos(deals.keys()).await?;
        tracing::debug!(target: "chain-connector", "get_deals: Got {} deals infos: {:?}", infos.len(), infos);
        let deals = infos
            .into_iter()
            .zip(deals)
            .map(|(details, (deal_id, mut workers))| match details {
                Ok((status, app_cid)) => {
                    if let Some((onchain_worker_id, cu_ids)) = workers.pop_first() {
                        DealResult::new(
                            deal_id,
                            DealInfo {
                                cu_ids,
                                status,
                                app_cid,
                                onchain_worker_id,
                            },
                        )
                    } else {
                        DealResult::with_error(deal_id, "No CUs are found for the deal".into())
                    }
                }
                Err(err) => DealResult::with_error(deal_id, err.to_string()),
            })
            .collect::<_>();
        tracing::debug!(target: "chain-connector", "get_deals: Return deals: {:?}", deals);
        Ok(deals)
    }

    async fn get_deal_infos<'a, I>(
        &self,
        deal_ids: I,
    ) -> Result<Vec<Result<(Deal::Status, String)>>>
    where
        I: IntoIterator<Item = &'a DealId> + ExactSizeIterator,
    {
        let mut batch = BatchRequestBuilder::new();
        let deal_count = deal_ids.len();
        for deal_id in deal_ids {
            let status_data = Deal::getStatusCall {}.abi_encode();
            batch.insert(
                "eth_call",
                rpc_params![EthCall::to(&status_data, &deal_id.to_address()), "latest"],
            )?;
            let app_cid_data = Deal::appCIDCall {}.abi_encode();
            batch.insert(
                "eth_call",
                rpc_params![EthCall::to(&app_cid_data, &deal_id.to_address()), "latest"],
            )?;
        }
        tracing::debug!(target: "chain-connector", "Batched get_deal_info request: {batch:?}");
        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        tracing::debug!(target: "chain-connector", "Batched get_deal_info response: {resp:?}");

        debug_assert_eq!(
            resp.len(),
            deal_count * 2,
            "JSON RPC Response contains not enough replies for the request, reposne = {resp:?}"
        );

        // Here we unite two responses with status and app cid for one deal.
        //
        // Since we put in the batch request requests in order [deal status, deal app_cid, ..]
        // we must get a reply in the same order.
        //
        // Note that JSON RPC specification says that it's a SHOULD not a MUST that the reply for
        // an ID will come at all, so this code can break if RPC malfunctions.
        let deal_info = resp
            .into_iter()
            .array_chunks::<2>()
            .map(|[status_resp, app_cid_resp]| try {
                let status = Deal::Status::from_hex(&status_resp?)?;
                let app_cid = CIDV1::from_hex(&app_cid_resp?)?.to_ipld()?;
                (status, app_cid)
            })
            .collect::<_>();
        Ok(deal_info)
    }

    async fn get_tx_nonce(&self) -> Result<U256> {
        let address = self.config.wallet_key.to_address().to_string();
        let resp: String = process_response(
            self.client
                .request("eth_getTransactionCount", rpc_params![address, "pending"])
                .await,
        )?;

        let nonce = U256::from_str(&resp).map_err(|err| InvalidU256(resp, err.to_string()))?;
        Ok(nonce)
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U256> {
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

    async fn estimate_gas_limit(&self, data: &[u8], to: &str) -> Result<U256> {
        let resp: String = process_response(
            self.client
                .request(
                    "eth_estimateGas",
                    rpc_params![json!({
                        "from": self.config.wallet_key.to_address().to_string(),
                        "to": to,
                        "data": encode_hex_0x(data),
                    })],
                )
                .await,
        )?;
        let limit = U256::from_str(&resp).map_err(|err| InvalidU256(resp, err.to_string()))?;
        Ok(limit)
    }

    pub async fn send_tx(&self, data: Vec<u8>, to: &str) -> Result<String> {
        let base_fee = self.get_base_fee_per_gas().await?;
        tracing::info!(target: "chain-connector", "Estimating gas for tx from {} to {} data {}", self.config.wallet_key.to_address(), to, encode_hex_0x(&data));
        let gas_limit = self.estimate_gas_limit(&data, to).await?;
        let max_priority_fee_per_gas = self.max_priority_fee_per_gas().await?;
        // (base fee + priority fee).
        let max_fee_per_gas = base_fee + max_priority_fee_per_gas;

        // We use this lock no ensure that we don't send two transactions with the same nonce
        let mut nonce_guard = self.tx_nonce_mutex.lock().await;
        let nonce = match *nonce_guard {
            None => self.get_tx_nonce().await?,
            Some(n) => n,
        };

        tracing::info!(target: "chain-connector",
            "Sending tx to {to} from {} data {}",
            self.config.wallet_key.to_address(), encode_hex_0x(&data)
        );

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

        let signed_tx = tx
            .sign(&self.config.wallet_key, Some(self.config.network_id))
            .to_bytes();
        let signed_tx = encode_hex_0x(signed_tx);

        let result: Result<String> = process_response(
            self.client
                .request("eth_sendRawTransaction", rpc_params![signed_tx])
                .await,
        );

        match result {
            Ok(resp) => {
                let new_nonce = nonce + Uint::from(1);
                *nonce_guard = Some(new_nonce);
                tracing::debug!(target: "chain-connector", "Incrementing nonce. New nonce {new_nonce}");
                Ok(resp)
            }
            Err(err) => {
                tracing::warn!(target: "chain-connector", "Failed to send tx: {err}, resetting nonce");
                *nonce_guard = None;
                Err(err)
            }
        }
    }

    pub async fn register_worker(
        &self,
        deal_id: &DealId,
        worker_id: WorkerId,
        onchain_worker_id: OnChainWorkerId,
    ) -> Result<String> {
        let data = Deal::activateWorkerCall {
            onchainId: FixedBytes::from_slice(&onchain_worker_id),
            offchainId: peer_id_to_bytes(worker_id.into()).into(),
        }
        .abi_encode();
        tracing::debug!(target: "chain-connector", "Registering worker {worker_id} for deal {deal_id} with onchain_id {}", encode_hex_0x(onchain_worker_id));
        self.send_tx(data, deal_id.as_str()).await
    }

    fn difficulty_params(&self) -> ArrayParams {
        let data = Core::difficultyCall {}.abi_encode();

        self.make_latest_diamond_rpc_params(data)
    }

    pub async fn get_deal_workers(&self, deal_id: &DealId) -> Result<Vec<Deal::Worker>> {
        let data = Deal::getWorkersCall {}.abi_encode();
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![EthCall::to(&data, &deal_id.to_address()), "latest"],
                )
                .await,
        )?;
        let bytes = decode_hex(&resp)?;
        let workers = <Array<Deal::Worker> as SolType>::abi_decode(&bytes, true)?;

        Ok(workers)
    }

    fn init_timestamp_params(&self) -> ArrayParams {
        let data = Core::initTimestampCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }
    fn global_nonce_params(&self) -> ArrayParams {
        let data = Capacity::getGlobalNonceCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }
    fn current_epoch_params(&self) -> ArrayParams {
        let data = Core::currentEpochCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }
    fn epoch_duration_params(&self) -> ArrayParams {
        let data = Core::epochDurationCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }

    fn min_proofs_per_epoch_params(&self) -> ArrayParams {
        let data = Core::minProofsPerEpochCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }

    fn max_proofs_per_epoch_params(&self) -> ArrayParams {
        let data = Core::maxProofsPerEpochCall {}.abi_encode();
        self.make_latest_diamond_rpc_params(data)
    }

    fn make_latest_diamond_rpc_params(&self, data: Vec<u8>) -> ArrayParams {
        rpc_params![
            EthCall::to(&data, &self.config.diamond_contract_address),
            "latest"
        ]
    }
}

#[async_trait]
impl ChainConnector for HttpChainConnector {
    async fn get_current_commitment_id(&self) -> Result<Option<CommitmentId>> {
        let peer_id = peer_id_to_bytes(self.host_id);
        let data = Offer::getComputePeerCall {
            peerId: peer_id.into(),
        }
        .abi_encode();
        let resp: String = process_response(
            self.client
                .request("eth_call", self.make_latest_diamond_rpc_params(data))
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
        batch.insert("eth_getBlockByNumber", rpc_params!["latest", false])?;

        let resp: BatchResponse<Value> = self.client.batch_request(batch).await?;
        tracing::debug!(target: "chain-connector", "Got cc init params response: {resp:?}");

        let mut results = resp
            .into_ok()
            .map_err(|err| eyre!("Some request failed in a batch {err:?}"))?;

        // TODO: check with 0x and write test
        let difficulty: FixedBytes<32> =
            FixedBytes::from_str(&parse_str_field(results.next(), "difficulty")?)?;

        let init_timestamp = U256::from_str(&parse_str_field(results.next(), "init_timestamp")?)?;

        let global_nonce = FixedBytes::from_str(&parse_str_field(results.next(), "global_nonce")?)?;

        let current_epoch = U256::from_str(&parse_str_field(results.next(), "current_epoch")?)?;

        let epoch_duration = U256::from_str(&parse_str_field(results.next(), "epoch_duration")?)?;

        let min_proofs_per_epoch =
            U256::from_str(&parse_str_field(results.next(), "min_proofs_per_epoch")?)?;

        let max_proofs_per_epoch =
            U256::from_str(&parse_str_field(results.next(), "max_proofs_per_epoch")?)?;

        let header = BlockHeader::from_json(
            results
                .next()
                .ok_or_else(|| eyre!("Block header not found in response"))?,
        )?;

        Ok(CCInitParams {
            difficulty: Difficulty::new(difficulty.0),
            init_timestamp,
            current_timestamp: header.timestamp,
            global_nonce: GlobalNonce::new(global_nonce.0),
            current_epoch,
            epoch_duration,
            min_proofs_per_epoch,
            max_proofs_per_epoch,
        })
    }

    async fn get_compute_units(&self) -> Result<Vec<ComputeUnit>> {
        let data = Offer::getComputeUnitsCall {
            peerId: peer_id_to_bytes(self.host_id).into(),
        }
        .abi_encode();

        let resp: String = process_response(
            self.client
                .request("eth_call", self.make_latest_diamond_rpc_params(data))
                .await,
        )?;
        let bytes = decode_hex(&resp)?;
        let compute_units = <Array<ComputeUnit> as SolType>::abi_decode(&bytes, true)?;

        Ok(compute_units)
    }

    async fn get_commitment_status(&self, commitment_id: CommitmentId) -> Result<CCStatus> {
        let data = Capacity::getStatusCall {
            commitmentId: commitment_id.0.into(),
        }
        .abi_encode();

        let resp: String = process_response(
            self.client
                .request("eth_call", self.make_latest_diamond_rpc_params(data))
                .await,
        )?;
        Ok(<CCStatus as SolType>::abi_decode(
            &decode_hex(&resp)?,
            true,
        )?)
    }

    async fn get_global_nonce(&self) -> Result<GlobalNonce> {
        let resp: String = process_response(
            self.client
                .request("eth_call", self.global_nonce_params())
                .await,
        )?;

        let bytes: FixedBytes<32> = FixedBytes::from_str(&resp)?;
        Ok(GlobalNonce::new(bytes.0))
    }

    async fn submit_proofs(
        &self,
        unit_ids: Vec<CUID>,
        local_nonces: Vec<LocalNonce>,
        result_hashes: Vec<ResultHash>,
    ) -> Result<String> {
        let data = Capacity::submitProofsCall {
            unitIds: unit_ids.into_iter().map(|id| id.as_ref().into()).collect(),
            localUnitNonces: local_nonces
                .into_iter()
                .map(|n| n.as_ref().into())
                .collect(),

            resultHashes: result_hashes
                .into_iter()
                .map(|hash| hash.as_ref().into())
                .collect(),
        }
        .abi_encode();

        self.send_tx(data, &self.config.diamond_contract_address)
            .await
    }

    async fn get_deal_statuses(&self, deal_ids: Vec<DealId>) -> Result<Vec<Result<Deal::Status>>> {
        let mut batch = BatchRequestBuilder::new();
        for deal_id in deal_ids {
            let data = Deal::getStatusCall {}.abi_encode();
            batch.insert(
                "eth_call",
                rpc_params![EthCall::to(&data, &deal_id.to_address()), "latest"],
            )?;
        }

        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let statuses = resp
            .into_iter()
            .map(|status_resp| {
                status_resp
                    .map(|status| Deal::Status::from_hex(&status))
                    .map_err(Into::into)
                    .flatten()
            })
            .collect::<_>();

        Ok(statuses)
    }
    async fn exit_deal(
        &self,
        deal_id: &DealId,
        onchain_worker_id: OnChainWorkerID,
    ) -> Result<String> {
        let data = Deal::removeWorkerCall {
            onchainId: onchain_worker_id,
        }
        .abi_encode();

        self.send_tx(data, &deal_id.to_address()).await
    }

    async fn get_tx_statuses(&self, tx_hashes: Vec<String>) -> Result<Vec<Result<Option<bool>>>> {
        let receipts = self.get_tx_receipts(tx_hashes).await?;

        let statuses = receipts
            .into_iter()
            .map(|receipt| match receipt {
                Ok(TxStatus::Pending) => Ok(None),
                Ok(TxStatus::Processed(receipt)) => Ok(Some(receipt.is_ok)),
                Err(err) => Err(err),
            })
            .collect();

        Ok(statuses)
    }

    async fn get_tx_receipts(&self, tx_hashes: Vec<String>) -> Result<Vec<Result<TxStatus>>> {
        let mut batch = BatchRequestBuilder::new();
        for tx_hash in tx_hashes {
            batch.insert("eth_getTransactionReceipt", rpc_params![tx_hash])?;
        }
        let resp: BatchResponse<Value> = self.client.batch_request(batch).await?;

        let receipts = resp
            .into_iter()
            .map(|receipt| try {
                match serde_json::from_value::<Option<RawTxReceipt>>(receipt?)? {
                    // When there's no receipt yet, the transaction is considered pending
                    None => TxStatus::Pending,
                    Some(raw_receipt) => TxStatus::Processed(raw_receipt.to_tx_receipt()),
                }
            })
            .collect();

        Ok(receipts)
    }
}

fn parse_str_field(value: Option<Value>, field: &'static str) -> Result<String> {
    value
        .ok_or_else(|| FieldNotFound(field))?
        .as_str()
        .ok_or_else(|| ResponseParseError(format!("Field {} is not a string", field)))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {

    use alloy_primitives::U256;
    use alloy_primitives::{hex, uint};
    use alloy_sol_types::sol_data::Array;
    use alloy_sol_types::SolType;
    use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash, CUID};
    use clarity::PrivateKey;
    use mockito::Matcher;
    use serde::Deserialize;
    use serde_json::json;
    use std::assert_matches::assert_matches;
    use std::str::FromStr;
    use std::sync::Arc;

    use chain_data::peer_id_from_hex;
    use fluence_libp2p::RandomPeerId;
    use hex_utils::{decode_hex, encode_hex_0x};
    use log_utils::{enable_logs_for, LogSpec};

    use crate::connector::TxStatus;
    use crate::Deal::Status::ACTIVE;
    use crate::{
        is_commitment_not_active, CCStatus, ChainConnector, CommitmentId, ConnectorError,
        HttpChainConnector,
    };

    fn get_connector(url: &str) -> Arc<HttpChainConnector> {
        let (connector, _) = HttpChainConnector::new(
            server_config::ChainConfig {
                http_endpoint: url.to_string(),
                diamond_contract_address: "0x2f5224b7Cb8bd98d9Ef61c247F4741758E8E873d".to_string(),
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
        let cu_1 = crate::Offer::ComputeUnit {
            id: hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5").into(),
            deal: Default::default(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let cu_2 = crate::Offer::ComputeUnit {
            id: hex!("ba3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d1").into(),
            deal: Default::default(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let expected_data = encode_hex_0x(Array::<crate::Offer::ComputeUnit>::abi_encode(&vec![
            cu_1.clone(),
            cu_2.clone(),
        ]));
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new_async().await;
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

        let mut server = mockito::Server::new_async().await;
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

        let mut server = mockito::Server::new_async().await;
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
        let mut server = mockito::Server::new_async().await;
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
          },
          {
              "jsonrpc": "2.0",
              "result": {
                "hash": "0x402c63844e7797c56468e5c9ca241d7f99b102c6e683fd371c1558fc87ff0963",
                "parentHash": "0x4904bfa81f0c577da1caa89826c3cf05a952e51dd39226709ed643c0f3847992",
                "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
                "miner": "0x5d159e79d541c35d68d1c8a5da02637fff779da0",
                "stateRoot": "0x5f928ec52b4c7d259e0cc744f2be0e17952a94e721b732d03b91142ab23bd497",
                "transactionsRoot": "0xc6d90adaa66f9e4e53d27c59c946f8f1b6aa9d1092a414c2290c05a4e081a8e5",
                "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                "number": "0x8b287",
                "gasUsed": "0x8af3a5a",
                "gasLimit": "0xadb3bb8",
                "extraData": "0x",
                "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                "timestamp": "0x666c79b1",
                "difficulty": "0x0",
                "totalDifficulty": "0x0",
                "sealFields": [],
                "uncles": [],
                "transactions": [
                  {
                    "accessList": [],
                    "blockHash": "0x402c63844e7797c56468e5c9ca241d7f99b102c6e683fd371c1558fc87ff0963",
                    "blockNumber": "0x8b287",
                    "chainId": "0x8613d62c79827",
                    "from": "0xb3b172cc702e3ae32ce0c73a050037a7750e41a6",
                    "gas": "0xadb3bb8",
                    "gasPrice": "0x64",
                    "hash": "0xdd5a4b397f222a9432b3d592ecca96171860953c5d2630f5f1f9f3614fdd8cd5",
                    "input": "0x4ece5685649cea1e34fec8ae2f5f8195124ec9c9e268b333f51d38dd12c9c89b026dd66ea6f4997b286ae1873a26c2f8cac3980af7ac51b185c9754d3a116d8a3c3a02fc0001286ad1ddc35083d1773e4f451e27047b3cdb5a1ebc592fd521de7780eb4c",
                    "maxFeePerGas": "0x64",
                    "maxPriorityFeePerGas": "0x0",
                    "nonce": "0x34d6",
                    "r": "0xfceb7aaceb1dc56f1e0b33584361c17ba1d8d79a5ec378c81d789c83e5eb7016",
                    "s": "0x13ba30819306a8cb71c60261824a77e29a0571514ffbb3f08f8503303caec56d",
                    "to": "0x066d0e888b62b7c2571cf867d2b26d6afefac720",
                    "transactionIndex": "0x0",
                    "type": "0x2",
                    "v": "0x1",
                    "value": "0x0"
                  }
                ],
                "size": "0xf7",
                "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "nonce": "0x0000000000000000",
                "baseFeePerGas": "0x64"
              },
              "id": 7
            }
        ]"#;

        let mut server = mockito::Server::new_async().await;
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
            <Difficulty>::from_str(
                "76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec"
            )
            .unwrap()
        );
        assert_eq!(init_params.init_timestamp, uint!(1707760129_U256));
        assert_eq!(
            init_params.global_nonce,
            <GlobalNonce>::from_str(
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

        let mut server = mockito::Server::new_async().await;
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

        let cu_ids = vec![CUID::new([0u8; 32])];
        let local_nonces = vec![LocalNonce::new([0u8; 32])];
        let result_hashes = vec![ResultHash::from_slice([0u8; 32])];

        let result = get_connector(&url)
            .submit_proofs(cu_ids, local_nonces, result_hashes)
            .await;

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
        let mut server = mockito::Server::new_async().await;
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

        let cu_ids = vec![CUID::new([0u8; 32])];
        let local_nonces = vec![LocalNonce::new([0u8; 32])];
        let result_hashes = vec![ResultHash::from_slice([0u8; 32])];

        let result = get_connector(&url)
            .submit_proofs(cu_ids, local_nonces, result_hashes)
            .await
            .unwrap();

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

        let mut server = mockito::Server::new_async().await;
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
        enable_logs_for(LogSpec::new(vec!["chain-connector=debug".parse().unwrap()]));

        let cu_1 = crate::Offer::ComputeUnit {
            id: hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5").into(),
            deal: hex!("5e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c").into(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let cu_2 = crate::Offer::ComputeUnit {
            id: hex!("ba3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d1").into(),
            deal: hex!("6e3d0fde6f793b3115a9e7f5ebc195bbeed35d6d").into(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let compute_units_response =
            encode_hex_0x(Array::<crate::Offer::ComputeUnit>::abi_encode(&vec![
                cu_1.clone(),
                cu_2.clone(),
            ]));
        let compute_units_response = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": compute_units_response,
        });

        let expected_app_cid = "bafkreiekvwp2w7t7vw4jzjq4s4n4wc323c6dnexmy4axh6c7tiza5wxzm4";
        let deal_status_response =
            "0x0000000000000000000000000000000000000000000000000000000000000001";
        let app_cid_response = "0x01551220000000000000000000000000000000000000000000000000000000008aad9fab7e7fadb89ca61c971bcb0b7ad8bc3692ecc70173f85f9a320edaf967";
        let deal_info_response = json!([
            {
                "jsonrpc": "2.0",
                "id": 1,
                "result": deal_status_response,
            },
            {
                "jsonrpc": "2.0",
                "id": 2,
                "result": app_cid_response
            },
            {
                "jsonrpc": "2.0",
                "id": 3,
                "result": deal_status_response,
            },
            {
                "jsonrpc": "2.0",
                "id": 4,
                "result": app_cid_response
            }

        ]);

        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            .expect(2)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                match body {
                    serde_json::Value::Object(_) => {
                        compute_units_response.clone().to_string().into()
                    }
                    serde_json::Value::Array(_) => deal_info_response.to_string().into(),
                    x => {
                        panic!("unexpected body: {x:?}");
                    }
                }
            })
            .create();

        let deals = get_connector(&url).get_deals().await.unwrap();

        assert_eq!(deals.len(), 2, "there should be only two deals: {deals:?}");
        assert!(deals[0].success, "failed to get a deal: {deals:?}");
        assert_eq!(deals[0].deal_id, cu_1.deal.to_string());

        let deal_info = &deals[0].deal_info[0];
        assert_eq!(deal_info.status, ACTIVE);
        assert_eq!(
            deal_info.cu_ids.len(),
            1,
            "there should be only one unit id: {deals:?}"
        );
        assert_eq!(deal_info.cu_ids[0], CUID::new(cu_1.id.0));
        assert_eq!(deal_info.app_cid, expected_app_cid);

        // Second deal
        assert!(deals[1].success, "failed to get a deal: {deals:?}");
        assert_eq!(deals[1].deal_id, cu_2.deal.to_string());

        let deal_info = &deals[1].deal_info[0];
        assert_eq!(deal_info.status, ACTIVE);
        assert_eq!(
            deal_info.cu_ids.len(),
            1,
            "there should be only one unit id: {deals:?}"
        );
        assert_eq!(deal_info.cu_ids[0], CUID::new(cu_2.id.0));
        assert_eq!(deal_info.app_cid, expected_app_cid);

        mock.assert();
    }

    #[tokio::test]
    async fn test_get_deals_no_deals() {
        let cu_1 = crate::Offer::ComputeUnit {
            id: hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5").into(),
            deal: Default::default(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let cu_2 = crate::Offer::ComputeUnit {
            id: hex!("ba3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d1").into(),
            deal: Default::default(),
            startEpoch: Default::default(),
            onchainWorkerId: Default::default(),
        };

        let compute_units_response =
            encode_hex_0x(Array::<crate::Offer::ComputeUnit>::abi_encode(&vec![
                cu_1.clone(),
                cu_2.clone(),
            ]));
        let compute_units_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{compute_units_response}\",\"id\":0}}");
        let mut server = mockito::Server::new_async().await;
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
        let onchain_worker_id =
            decode_hex("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5").unwrap();

        let mut server = mockito::Server::new_async().await;
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
            .register_worker(&deal_id, worker_id, onchain_worker_id)
            .await
            .unwrap();
        assert_eq!(result, expected_tx_hash);

        mock.assert();
    }

    #[tokio::test]
    async fn test_get_receipts_several() {
        let tx_hash =
            "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05".to_string();
        let receipt_response = r#"
            [{
                "id": 0,
                "jsonrpc": "2.0",
                "result": null
            },
            {
                "id": 1,
                "jsonrpc": "2.0",
                "result": {
                    "blockNumber": "0x123",
                    "status": "0x1",
                    "transactionHash": "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
                }
            },
            {
                "id": 2,
                "jsonrpc": "2.0",
                "error": {
                    "code": -32000,
                    "message": "some error"
                }
            }]
        "#;
        let mut server = mockito::Server::new_async().await;
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
        assert_eq!(result.len(), 3);

        let pending = result.remove(0);
        assert!(pending.is_ok(), "should get pending status: {:?}", pending);
        assert_matches!(pending.unwrap(), TxStatus::Pending);

        let ok = result.remove(0);
        assert!(ok.is_ok(), "should get a receipt: {:?}", ok);
        assert_matches!(ok.unwrap(), TxStatus::Processed(_));

        assert!(result[0].is_err(), "should be error: {:?}", result[0]);

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
        let mut server = mockito::Server::new_async().await;
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
        assert_matches!(result, TxStatus::Processed(_));
        if let TxStatus::Processed(receipt) = result {
            assert_eq!(receipt.transaction_hash, tx_hash);
        }

        mock.assert();
    }
}
