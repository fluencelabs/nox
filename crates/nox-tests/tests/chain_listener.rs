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
#![feature(try_blocks)]

use std::collections::HashMap;
use std::collections::{BTreeSet, VecDeque};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use alloy_primitives::fixed_bytes;
use alloy_primitives::Address;
use alloy_primitives::FixedBytes;
use alloy_primitives::U256;
use alloy_sol_types::sol_data::Array;
use alloy_sol_types::SolInterface;
use alloy_sol_types::SolType;
use alloy_sol_types::SolValue;
use alloy_sol_types::{SolCall, SolEvent};
use ccp_rpc_client::CCPRpcServer;
use ccp_rpc_client::OrHex;
use ccp_shared::proof::CCProof;
use ccp_shared::proof::ProofIdx;
use ccp_shared::types::Difficulty;
use ccp_shared::types::GlobalNonce;
use ccp_shared::types::LogicalCoreId;
use ccp_shared::types::PhysicalCoreId;
use ccp_shared::types::CUID;
use clarity::{PrivateKey, Transaction};
use eyre::{eyre, OptionExt};
use futures::StreamExt;
use hex::FromHex;
use itertools::Itertools;
use jsonrpsee::core::async_trait;
use jsonrpsee::server::ServerHandle;
use jsonrpsee::types::ErrorObject;
use jsonrpsee::types::ErrorObjectOwned;
use jsonrpsee::RpcModule;
use jsonrpsee::SubscriptionMessage;
use libp2p::PeerId;
use maplit::hashmap;
use parking_lot::{Mutex, MutexGuard};
use serde_json::json;
use serde_json::Value;
use tempfile::TempDir;
use tokio_stream::wrappers::BroadcastStream;

use chain_connector::Capacity::CapacityCalls;
use chain_connector::CommitmentId;
use chain_connector::Core::CoreCalls;
use chain_connector::Deal::DealCalls;
use chain_connector::Deal::Status;
use chain_connector::Offer::ComputePeer;
use chain_connector::Offer::ComputeUnit;
use chain_connector::Offer::OfferCalls;
use chain_connector::{CCStatus, Offer};
use chain_data::parse_peer_id;
use chain_data::peer_id_to_bytes;
use chain_listener::CommitmentActivated;
use chain_listener::ComputeUnitMatched;
use chain_listener::UnitDeactivated;
use chain_listener::CIDV1;
use created_swarm::make_swarms_with_cfg;
use created_swarm::CreatedSwarm;
use log_utils::enable_logs;
use server_config::ChainConfig;
use server_config::ChainListenerConfig;

struct ChainServer {
    address: String,
    cc_contract_address: String,
    market_contract_address: String,
    _handle: ServerHandle,
    new_heads_sender: tokio::sync::broadcast::Sender<SubscriptionMessage>,
    logs_sender: tokio::sync::broadcast::Sender<LogsParams>,
    chain_state: Arc<Mutex<ChainState>>,
}

impl ChainServer {
    pub async fn new(
        cc_contract_address: String,
        core_contract_address: String,
        market_contract_address: String,
        init_state: ChainState,
    ) -> eyre::Result<Self> {
        let server = jsonrpsee::server::Server::builder()
            .set_message_buffer_capacity(10)
            .build("127.0.0.1:0")
            .await?;
        let (new_heads_sender, _new_heads_receiver) = tokio::sync::broadcast::channel(16);
        let (logs_sender, _logs_receiver) = tokio::sync::broadcast::channel(16);
        let chain_state = Arc::new(Mutex::new(init_state));
        let mut module = RpcModule::new(Context {
            new_heads_sender: new_heads_sender.clone(),
            logs_sender: logs_sender.clone(),
            chain_state: chain_state.clone(),
        });

        let inner_cc_contract_address = cc_contract_address.clone();
        module.register_method("eth_estimateGas", |_params, _ctx| {
            Ok::<String, ErrorObject>(to_hex_with_prefix(U256::from(200).abi_encode()))
        })?;
        module.register_method("eth_maxPriorityFeePerGas", |_params, _ctx| {
            Ok::<String, ErrorObject>(to_hex_with_prefix(U256::from(200).abi_encode()))
        })?;
        module.register_method("eth_getTransactionCount", |_params, _ctx| {
            Ok::<String, ErrorObject>(to_hex_with_prefix(U256::from(1).abi_encode()))
        })?;
        module.register_method("eth_getBlockByNumber", |_params, ctx| {
            let state = ctx.chain_state.lock();
            Ok::<Value, ErrorObject>(json!({
                "number": state.block_number,
                "baseFeePerGas": U256::from(200)
            }))
        })?;
        let inner_market_contract_address = market_contract_address.clone();
        module.register_method("eth_sendRawTransaction", move |params, ctx| {
            let params = params.one::<String>();
            let sub = params.map_err(|_| ErrorObject::owned(500, "", None::<String>))?;

            let transaction = hex::decode(&sub[2..sub.len()])
                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;

            let transaction = Transaction::decode_from_rlp(&mut transaction.as_slice())
                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;

            match transaction {
                Transaction::Legacy { .. } => {}
                Transaction::Eip2930 { .. } => {}
                Transaction::Eip1559 { to, data, .. } => {
                    let to = to.to_string();
                    if to == inner_market_contract_address {
                        let call =
                            Offer::returnComputeUnitFromDealCall::abi_decode(data.as_slice(), true)
                                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;

                        let state = ctx.chain_state.lock();
                        let unit_state = state
                            .unit_state
                            .get(&call.unitId)
                            .ok_or(ErrorObject::owned(500, "", None::<String>))?;

                        let commitment_id = unit_state
                            .commitment_id
                            .clone()
                            .ok_or(ErrorObject::owned(500, "", None::<String>))?;

                        let event = CommitmentActivated {
                            peerId: FixedBytes::new(peer_id_to_bytes(unit_state.peer_id)),
                            commitmentId: FixedBytes::new(commitment_id.0),
                            startEpoch: state.current_epoch + U256::from(1),
                            endEpoch: state.current_epoch + U256::from(100),
                            unitIds: vec![call.unitId],
                        };

                        let data = event.encode_data();
                        let data = to_hex_with_prefix(data);

                        let message = SubscriptionMessage::from_json(&json!({
                          "topics": vec![
                            CommitmentActivated::SIGNATURE_HASH.to_string(),
                            to_hex_with_prefix(peer_id_to_bytes(unit_state.peer_id)),
                            commitment_id.to_string()
                          ],
                          "data": data,
                          "blockNumber": state.block_number,
                          "removed": false
                        }))
                        .unwrap();

                        let message = LogsParams {
                            address: inner_cc_contract_address.clone(),
                            topics: vec![
                                CommitmentActivated::SIGNATURE_HASH.to_string(),
                                to_hex_with_prefix(peer_id_to_bytes(unit_state.peer_id)),
                            ],
                            message,
                        };

                        ctx.logs_sender.send(message).unwrap();
                    } else {
                        todo!()
                    }
                }
            }
            Ok::<String, ErrorObject>("done".to_string())
        })?;

        let inner_cc_contract_address = cc_contract_address.clone();
        let inner_market_contract_address = market_contract_address.clone();

        module.register_method("eth_call", move |params, ctx| {
            let params: eyre::Result<(String, Vec<u8>)> = try {
                let params = params.parse::<Value>()?;
                let array = params.as_array().ok_or_eyre("Not array")?;
                let head = array.first().ok_or_eyre("Empty array")?;
                let object = head.as_object().ok_or_eyre("Not object")?;
                let to = object.get("to").ok_or_eyre("To not found")?;
                let data = object.get("data").ok_or_eyre("Data not found")?;
                let to = to.as_str().ok_or_eyre("Not string")?;
                let data = data.as_str().ok_or_eyre("Not string")?;
                let data = hex::decode(data)?;
                (to.to_string(), data)
            };
            let (to, data) = params.map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
            let state = ctx.chain_state.lock();
            let deal_addresses = state
                .deal_statuses
                .keys()
                .map(|addr| format!("{:?}", addr))
                .collect::<BTreeSet<_>>();

            let result: Result<String, ErrorObject> = match to {
                to if to == core_contract_address => {
                    let res = CoreCalls::abi_decode(data.as_slice(), true)
                        .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                    match res {
                        CoreCalls::capacity(_) => {
                            todo!()
                        }
                        CoreCalls::market(_) => {
                            todo!()
                        }
                        CoreCalls::currentEpoch(_) => {
                            Ok(to_hex_with_prefix(state.current_epoch.abi_encode()))
                        }
                        CoreCalls::epochDuration(_) => {
                            Ok(to_hex_with_prefix(state.epoch_duration.abi_encode()))
                        }
                        CoreCalls::initTimestamp(_) => {
                            Ok(to_hex_with_prefix(state.init_timestamp.abi_encode()))
                        }
                        CoreCalls::difficulty(_) => Ok(state.difficulty.to_string()),
                        CoreCalls::minProofsPerEpoch(_) => {
                            Ok(to_hex_with_prefix(state.min_proofs_per_epoch.abi_encode()))
                        }
                        CoreCalls::maxProofsPerEpoch(_) => {
                            Ok(to_hex_with_prefix(state.max_proofs_per_epoch.abi_encode()))
                        }
                    }
                }
                to if to == inner_cc_contract_address => {
                    let res = CapacityCalls::abi_decode(data.as_slice(), true)
                        .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                    match res {
                        CapacityCalls::getGlobalNonce(_) => Ok(state.global_nonce.to_string()),
                        CapacityCalls::getStatus(params) => {
                            let commitment_id = CommitmentId(params.commitmentId.0);
                            let status = state
                                .commitment_statuses
                                .get(&commitment_id)
                                .ok_or(ErrorObject::owned(500, "", None::<String>))?;
                            let data = &status.abi_encode();
                            Ok(to_hex_with_prefix(data))
                        }
                        CapacityCalls::submitProof(_) => {
                            todo!()
                        }
                    }
                }
                to if to == inner_market_contract_address => {
                    let res = OfferCalls::abi_decode(data.as_slice(), true)
                        .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                    match res {
                        OfferCalls::getComputePeer(params) => {
                            let peer_id = parse_peer_id(params.peerId.to_vec())
                                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                            let state = state.peer_states.get(&peer_id).ok_or(
                                ErrorObject::owned(33, "Peer doesn't exists", None::<String>),
                            )?;
                            let data = &state.compute_peer.abi_encode();
                            Ok(to_hex_with_prefix(data))
                        }
                        OfferCalls::getComputeUnits(params) => {
                            let peer_id = parse_peer_id(params.peerId.to_vec())
                                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                            let state = state
                                .peer_states
                                .get(&peer_id)
                                .ok_or(ErrorObject::owned(500, "", None::<String>))?;
                            let data = Array::<ComputeUnit>::abi_encode(&state.units);
                            Ok(to_hex_with_prefix(data))
                        }
                        OfferCalls::returnComputeUnitFromDeal(_params) => {
                            todo!()
                        }
                    }
                }
                to if deal_addresses.contains(&to) => {
                    let res = DealCalls::abi_decode(data.as_slice(), true)
                        .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                    match res {
                        DealCalls::getStatus(_) => {
                            let deal_address = Address::from_str(&to)
                                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                            let deal = state
                                .deal_statuses
                                .get(&deal_address)
                                .ok_or(ErrorObject::owned(500, "", None::<String>))?;
                            let data = deal.abi_encode();
                            Ok(to_hex_with_prefix(data))
                        }
                        DealCalls::appCID(_) => {
                            todo!()
                        }
                        DealCalls::setWorker(_) => {
                            todo!()
                        }
                    }
                }
                _ => Err(ErrorObject::owned(500, "", None::<String>)),
            };
            result
        })?;
        module.register_subscription(
            "eth_subscribe",
            "eth_subscription",
            "eth_unsubscribe",
            |params, pending, ctx| async move {
                let mut params = params.sequence();
                if let Ok(sub) = params.next::<String>() {
                    let sink = pending.accept().await.unwrap();
                    let params = params.next::<Value>().ok();
                    tokio::spawn(async move {
                        let mut stream = match sub.as_str() {
                            "newHeads" => BroadcastStream::new(ctx.new_heads_sender.subscribe())
                                .filter_map(|el| async { el.ok() })
                                .boxed(),
                            "logs" => BroadcastStream::new(ctx.logs_sender.subscribe())
                                .filter_map(|el| {
                                    let params = params.clone();
                                    async move { filter_logs(el.ok(), params) }
                                })
                                .boxed(),
                            _ => todo!(),
                        };
                        loop {
                            if let Some(event) = stream.next().await {
                                sink.send(event).await.unwrap();
                            }
                        }
                    });
                }
                Ok(())
            },
        )?;

        let addr = server.local_addr()?;
        let handle = server.start(module);

        Ok(ChainServer {
            address: addr.to_string(),
            cc_contract_address,
            market_contract_address,
            _handle: handle,
            logs_sender,
            new_heads_sender,
            chain_state,
        })
    }

    pub async fn register_peer(&self, params: RegisterPeerParams) {
        let mut state = self.chain_state.lock();
        for (peer_id, data) in params.state {
            for unit in &data.units {
                state.unit_state.insert(
                    unit.id,
                    UnitState {
                        peer_id,
                        commitment_id: None,
                    },
                );
            }
            state.peer_states.insert(peer_id, data);
        }
    }

    pub async fn move_to_epoch(&self, epoch: U256) -> U256 {
        let mut state = self.chain_state.lock();
        let epoch_between = epoch - state.current_epoch;
        if epoch_between > U256::ZERO {
            let block_number = (epoch_between * state.epoch_duration) / state.block_duration;
            while state.block_number < block_number {
                self.send_next_block_with_state(&mut state);
                let current_epoch = state.current_epoch;
                let for_activation = state
                    .commitment_activation_at
                    .iter()
                    .filter_map(|(k, &v)| if v <= current_epoch { Some(k) } else { None })
                    .cloned()
                    .collect::<Vec<_>>();

                for commitment_id in for_activation {
                    state.commitment_activation_at.remove(&commitment_id);
                    state
                        .commitment_statuses
                        .insert(commitment_id, CCStatus::Active);
                }
            }
            state.block_number
        } else {
            state.block_number
        }
    }

    pub async fn send_next_block(&self) -> U256 {
        let mut state = self.chain_state.lock();
        self.send_next_block_with_state(&mut state)
    }

    pub fn send_next_block_with_state(&self, state: &mut MutexGuard<ChainState>) -> U256 {
        let next_block_number = state.block_number + U256::from(1);
        let timestamp = state.block_duration * next_block_number;
        let next_epoch = U256::from(1) + (timestamp - state.init_timestamp) / state.epoch_duration;
        let message = SubscriptionMessage::from_json(&json!({
                           "difficulty": "0x15d9223a23aa",
                           "extraData": "0xd983010305844765746887676f312e342e328777696e646f7773",
                           "gasLimit": "0x47e7c4",
                           "gasUsed": "0x38658",
                           "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                           "miner": "0xf8b483dba2c3b7176a3da549ad41a48bb3121069",
                           "nonce": "0x084149998194cc5f",
                           "number": next_block_number,
                           "parentHash": "0x7736fab79e05dc611604d22470dadad26f56fe494421b5b333de816ce1f25701",
                           "receiptRoot": "0x2fab35823ad00c7bb388595cb46652fe7886e00660a01e867824d3dceb1c8d36",
                           "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
                           "stateRoot": "0xb3346685172db67de536d8765c43c31009d0eb3bd9c501c9be3229203f15f378",
                           "timestamp": timestamp,
                           "transactionsRoot": "0x0167ffa60e3ebc0b080cdb95f7c0087dd6c0e61413140e39d94d3468d7c9689f"
        })).unwrap();
        self.new_heads_sender.send(message).unwrap();
        state.current_epoch = next_epoch;
        state.block_number = next_block_number;
        next_block_number
    }

    pub async fn create_commitment(&self, peer_id: PeerId, commitment_id: &CommitmentId) {
        let mut chain_state = self.chain_state.lock();
        chain_state
            .commitment_statuses
            .insert(commitment_id.clone(), CCStatus::WaitDelegation);
        let peer_state = chain_state.peer_states.get_mut(&peer_id).unwrap();
        peer_state.compute_peer.commitmentId = FixedBytes::new(commitment_id.0);
        for (_, state) in &mut chain_state.unit_state {
            if state.peer_id == peer_id {
                state.commitment_id = Some(commitment_id.clone())
            }
        }
    }

    pub async fn activate_commitment(
        &self,
        peer_id: PeerId,
        commitment_id: &CommitmentId,
        commitment_epoch_duration: u8,
    ) -> U256 {
        let mut state = self.chain_state.lock();
        state
            .commitment_statuses
            .insert(commitment_id.clone(), CCStatus::WaitStart);
        let start_epoch = state.current_epoch + U256::from(1);
        state
            .commitment_activation_at
            .insert(commitment_id.clone(), start_epoch);
        let peer_state = state.peer_states.get_mut(&peer_id).unwrap();
        let mut event_unit_ids = Vec::with_capacity(peer_state.units.len());
        for unit in peer_state.units.iter_mut() {
            unit.startEpoch = start_epoch;
            event_unit_ids.push(unit.id);
        }

        let event = CommitmentActivated {
            peerId: FixedBytes::new(peer_id_to_bytes(peer_id)),
            commitmentId: FixedBytes::new(commitment_id.0),
            startEpoch: start_epoch,
            endEpoch: start_epoch + U256::from(commitment_epoch_duration),
            unitIds: event_unit_ids,
        };

        let data = event.encode_data();
        let data = to_hex_with_prefix(data);

        let message = SubscriptionMessage::from_json(&json!({
          "topics": vec![
            CommitmentActivated::SIGNATURE_HASH.to_string(),
            to_hex_with_prefix(peer_id_to_bytes(peer_id)),
            commitment_id.to_string()
          ],
          "data": data,
          "blockNumber": state.block_number,
          "removed": false
        }))
        .unwrap();

        let message = LogsParams {
            address: self.cc_contract_address.clone(),
            topics: vec![
                CommitmentActivated::SIGNATURE_HASH.to_string(),
                to_hex_with_prefix(peer_id_to_bytes(peer_id)),
            ],
            message,
        };

        self.logs_sender.send(message).unwrap();
        start_epoch
    }

    pub async fn create_deal(&self, params: CreateDealParams) {
        let mut state = self.chain_state.lock();
        let block_number = state.block_number;
        state.deal_statuses.insert(params.deal_id, Status::ACTIVE);
        let peer_state = state.peer_states.get_mut(&params.peer_id).unwrap();
        if let Some(state_unit) = peer_state
            .units
            .iter_mut()
            .find(|state_unit| state_unit.id == params.unit.id)
        {
            state_unit.deal = params.deal_id;
        }

        {
            let event = UnitDeactivated {
                commitmentId: FixedBytes::new(params.commitment_id.0),
                unitId: params.unit.id,
            };

            let data = event.encode_data();
            let data = to_hex_with_prefix(data);

            let message = SubscriptionMessage::from_json(&json!({
              "topics": vec![
                    UnitDeactivated::SIGNATURE_HASH.to_string(),
                    params.commitment_id.to_string(),
                    to_hex_with_prefix(params.unit.id.0)
              ],
              "data": data,
              "blockNumber": block_number,
              "removed": false
            }))
            .unwrap();

            let message = LogsParams {
                address: self.cc_contract_address.clone(),
                topics: vec![
                    UnitDeactivated::SIGNATURE_HASH.to_string(),
                    params.commitment_id.to_string(),
                ],
                message,
            };
            self.logs_sender.send(message).unwrap();
        }

        {
            let event = ComputeUnitMatched {
                peerId: FixedBytes::new(peer_id_to_bytes(params.peer_id)),
                deal: params.deal_id,
                unitId: params.unit.id,
                dealCreationBlock: block_number,
                appCID: CIDV1 {
                    prefixes: Default::default(),
                    hash: Default::default(),
                },
            };
            let data = event.encode_data();
            let data = to_hex_with_prefix(data);
            let message = SubscriptionMessage::from_json(&json!({
              "topics": vec![
                    ComputeUnitMatched::SIGNATURE_HASH.to_string(),
                    to_hex_with_prefix(peer_id_to_bytes(params.peer_id)),
                ],
              "data": data,
              "blockNumber": block_number,
              "removed": false
            }))
            .unwrap();
            let message = LogsParams {
                address: self.market_contract_address.clone(),
                topics: vec![
                    ComputeUnitMatched::SIGNATURE_HASH.to_string(),
                    to_hex_with_prefix(peer_id_to_bytes(params.peer_id)),
                ],
                message,
            };
            self.logs_sender.send(message).unwrap();
        }
    }

    pub async fn change_deal_status(&self, deal: Address, status: Status) {
        let mut state = self.chain_state.lock();
        state.deal_statuses.insert(deal, status);
    }
}

fn filter_logs(
    logs_params: Option<LogsParams>,
    params: Option<Value>,
) -> Option<SubscriptionMessage> {
    let check = || -> eyre::Result<SubscriptionMessage> {
        let params = params.ok_or_eyre("Empty params")?;
        let logs_params = logs_params.ok_or_eyre("Empty params")?;
        let address_from_params = params.get("address").ok_or_eyre("Empty address")?;
        let address_from_params = address_from_params.as_str().ok_or_eyre("Not string")?;
        if logs_params.address != address_from_params {
            return Err(eyre!("Not our address"));
        };
        let topics_from_params = params.get("topics").ok_or_eyre("Empty address")?;
        let topics_from_params = topics_from_params.as_array().ok_or_eyre("Not array")?;

        let topics_from_params = topics_from_params
            .iter()
            .flat_map(|el| el.as_str().map(|str| str.to_string()))
            .collect::<Vec<_>>();

        if logs_params.topics != topics_from_params {
            return Err(eyre!("Not our topics"));
        }

        Ok(logs_params.message)
    };

    check().ok()
}

struct Context {
    new_heads_sender: tokio::sync::broadcast::Sender<SubscriptionMessage>,
    logs_sender: tokio::sync::broadcast::Sender<LogsParams>,
    chain_state: Arc<Mutex<ChainState>>,
}

#[derive(Clone, Debug)]
struct LogsParams {
    address: String,
    topics: Vec<String>,
    message: SubscriptionMessage,
}

struct PeerState {
    units: Vec<ComputeUnit>,
    compute_peer: ComputePeer,
}

struct ChainState {
    block_number: U256,
    block_duration: U256,
    difficulty: Difficulty,
    init_timestamp: U256,
    global_nonce: GlobalNonce,
    current_epoch: U256,
    epoch_duration: U256,
    min_proofs_per_epoch: U256,
    max_proofs_per_epoch: U256,
    commitment_statuses: HashMap<CommitmentId, CCStatus>,
    commitment_activation_at: HashMap<CommitmentId, U256>,
    peer_states: HashMap<PeerId, PeerState>,
    deal_statuses: HashMap<Address, Status>,
    unit_state: HashMap<FixedBytes<32>, UnitState>,
}

impl Default for ChainState {
    fn default() -> Self {
        Self {
            block_number: U256::from(0),
            block_duration: U256::from(100),
            difficulty: Difficulty::from_hex(
                "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            )
            .unwrap(),
            init_timestamp: U256::from(0),
            global_nonce: GlobalNonce::from_hex(
                "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a",
            )
            .unwrap(),
            current_epoch: U256::from(0),
            epoch_duration: U256::from(1000),
            min_proofs_per_epoch: U256::from(1),
            max_proofs_per_epoch: U256::from(3),
            commitment_statuses: Default::default(),
            peer_states: Default::default(),
            deal_statuses: Default::default(),
            commitment_activation_at: Default::default(),
            unit_state: Default::default(),
        }
    }
}

struct UnitState {
    peer_id: PeerId,
    commitment_id: Option<CommitmentId>,
}

struct CCPServer {
    address: String,
    _handle: ServerHandle,
    active_commitment_params: Arc<Mutex<VecDeque<ActiveCommitmentParams>>>,
}

impl CCPServer {
    pub async fn get_or_wait_active_commitment_params(&self) -> ActiveCommitmentParams {
        loop {
            let mut state = self.active_commitment_params.lock();
            match state.pop_front() {
                None => {
                    drop(state);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Some(value) => return value.clone(),
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ActiveCommitmentParams {
    global_nonce: OrHex<GlobalNonce>,
    difficulty: OrHex<Difficulty>,
    cu_allocation: HashMap<PhysicalCoreId, OrHex<CUID>>,
}

struct CCPRpcImpl {
    active_commitment_params: Arc<Mutex<VecDeque<ActiveCommitmentParams>>>,
}

#[async_trait]
impl CCPRpcServer for CCPRpcImpl {
    async fn on_active_commitment(
        &self,
        global_nonce: OrHex<GlobalNonce>,
        difficulty: OrHex<Difficulty>,
        cu_allocation: HashMap<PhysicalCoreId, OrHex<CUID>>,
    ) -> Result<(), ErrorObjectOwned> {
        let mut params = self.active_commitment_params.lock();
        params.push_back(ActiveCommitmentParams {
            global_nonce,
            difficulty,
            cu_allocation,
        });
        Ok(())
    }

    async fn on_no_active_commitment(&self) -> Result<(), ErrorObjectOwned> {
        Ok(())
    }

    async fn get_proofs_after(
        &self,
        _proof_idx: ProofIdx,
        _limit: usize,
    ) -> Result<Vec<CCProof>, ErrorObjectOwned> {
        Ok(vec![])
    }

    async fn realloc_utility_cores(&self, _utility_core_ids: Vec<LogicalCoreId>) {}
}

impl CCPServer {
    pub async fn new() -> eyre::Result<Self> {
        let server = jsonrpsee::server::Server::builder()
            .set_message_buffer_capacity(10)
            .build("127.0.0.1:0")
            .await?;

        let addr = server.local_addr()?;

        let active_commitment_params = Arc::new(Mutex::new(VecDeque::new()));

        let ccp_rpc_impl = CCPRpcImpl {
            active_commitment_params: active_commitment_params.clone(),
        };
        let handle = server.start(ccp_rpc_impl.into_rpc());

        Ok(CCPServer {
            address: addr.to_string(),
            _handle: handle,
            active_commitment_params,
        })
    }
}

struct RegisterPeerParams {
    state: HashMap<PeerId, PeerState>,
}

struct CreateDealParams {
    deal_id: Address,
    peer_id: PeerId,
    commitment_id: CommitmentId,
    unit: ComputeUnit,
}

fn to_hex_with_prefix<T: AsRef<[u8]>>(data: T) -> String {
    let hex_string = hex::encode(data);
    format!("0x{}", hex_string)
}

struct ChainListenerTestEntities {
    ccp_server: CCPServer,
    chain_server: ChainServer,
    swarms: Vec<CreatedSwarm>,
    _events_dir: TempDir,
}

impl ChainListenerTestEntities {
    pub async fn new() -> eyre::Result<Self> {
        let chain_server = ChainServer::new(
            "0x73e491D86aDBc4504156Ad04254F2Ba7Eb5935cA".to_string(),
            "0x6f3cd1237da812dc14ebe3408bE9717b4913e87c".to_string(),
            "0x3bD52336C1991750E033be3867b5524E66bb5559".to_string(),
            ChainState::default(),
        )
        .await?;

        let ccp_server = CCPServer::new().await?;
        let events_dir = TempDir::new()?;
        let cc_events_dir = events_dir.path().to_path_buf();
        let chain_server_address = chain_server.address.clone();
        let ccp_server_address = ccp_server.address.clone();
        let swarms = make_swarms_with_cfg(1, move |mut cfg| {
            cfg.chain_config = Some(ChainConfig {
                http_endpoint: format!("http://{}", chain_server_address),
                cc_contract_address: "0x73e491D86aDBc4504156Ad04254F2Ba7Eb5935cA".to_string(),
                core_contract_address: "0x6f3cd1237da812dc14ebe3408bE9717b4913e87c".to_string(),
                market_contract_address: "0x3bD52336C1991750E033be3867b5524E66bb5559".to_string(),
                network_id: 1,
                wallet_key: PrivateKey::from_str(
                    "0xfdc4ba94809c7930fe4676b7d845cbf8fa5c1beae8744d959530e5073004cf3f",
                )
                .unwrap(),
                default_base_fee: None,
                default_priority_fee: None,
            });
            cfg.chain_listener_config = Some(ChainListenerConfig {
                ws_endpoint: format!("ws://{}", chain_server_address),
                ccp_endpoint: Some(format!("http://{}", ccp_server_address)),
                proof_poll_period: Duration::from_secs(5),
            });

            cfg.cc_events_dir = Some(cc_events_dir.clone());
            cfg.mocked_topology_count = Some(128);
            cfg.metrics_enabled = true;
            cfg
        })
        .await;
        Ok(Self {
            ccp_server,
            chain_server,
            swarms,
            _events_dir: events_dir,
        })
    }
}
const WAIT_BLOCK_POLLING_INTERVAL: Duration = Duration::from_millis(100);

async fn wait_block_process_on_nox(swarm: &CreatedSwarm, block_number: U256) -> eyre::Result<()> {
    let http_endpoint = swarm.http_listen_addr;
    let http_client = &reqwest::Client::new();
    loop {
        let response = http_client
            .get(format!("http://{}/metrics", http_endpoint))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::OK {
            let body = response.text().await?;
            let lines: Vec<_> = body.lines().map(|s| Ok(s.to_owned())).collect();
            let metrics = prometheus_parse::Scrape::parse(lines.into_iter())?;
            let metric = metrics
                .samples
                .into_iter()
                .find(|el| el.metric.as_str() == "chain_listener_last_process_block")
                .map(|el| el.value);

            let metric = metric.ok_or(eyre!("metric not found"))?;
            let value = match metric {
                prometheus_parse::Value::Gauge(value) => U256::try_from(value)?,
                _ => return Err(eyre!("wrong metric format")),
            };

            if value == block_number {
                return Ok(());
            }
        }

        tokio::time::sleep(WAIT_BLOCK_POLLING_INTERVAL).await
    }
}

#[tokio::test]
async fn test_cc_activation_flow() {
    enable_logs();
    let test_entities = ChainListenerTestEntities::new().await.unwrap();

    let owner_address = Address::from(fixed_bytes!("9b5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let offer_id = fixed_bytes!("17376d336cc28febe907ebf32629d63717eb5d4b0e1bb0e3d5e7c72db6ad58e5");

    let commitment_id = CommitmentId::new(
        fixed_bytes!("58e279641b29d191462381628d407f2a5a08fa0620b9b12b45aecbbdb7aa0d20").0,
    )
    .unwrap();

    let compute_unit_id_1 =
        fixed_bytes!("ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");

    let compute_unit_1 = ComputeUnit {
        id: compute_unit_id_1,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
    };

    let compute_peer_1 = ComputePeer {
        offerId: offer_id,
        commitmentId: FixedBytes::<32>::ZERO,
        unitCount: U256::from(1),
        owner: owner_address,
    };

    test_entities
        .chain_server
        .register_peer(RegisterPeerParams {
            state: hashmap! {
               test_entities.swarms[0].peer_id => PeerState {
                    units: vec![compute_unit_1],
                    compute_peer: compute_peer_1
                },
            },
        })
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // Create commitment with status wait delegation
    test_entities
        .chain_server
        .create_commitment(test_entities.swarms[0].peer_id, &commitment_id)
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    let activation_epoch = test_entities
        .chain_server
        .activate_commitment(test_entities.swarms[0].peer_id, &commitment_id, 100)
        .await;
    //active commitment on epoch 3
    let block_number = test_entities
        .chain_server
        .move_to_epoch(activation_epoch)
        .await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    //assert CCP
    let result = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    assert_eq!(
        result.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );

    assert_allocation(
        result.cu_allocation,
        vec![(
            PhysicalCoreId::new(127),
            "ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
        )],
    );
}

#[tokio::test]
async fn test_deal_insufficient_funds_flow() {
    enable_logs();
    let test_entities = ChainListenerTestEntities::new().await.unwrap();

    let deal_address = Address::from(fixed_bytes!("9c5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let owner_address = Address::from(fixed_bytes!("9b5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let offer_id = fixed_bytes!("17376d336cc28febe907ebf32629d63717eb5d4b0e1bb0e3d5e7c72db6ad58e5");

    let commitment_id = CommitmentId::new(
        fixed_bytes!("58e279641b29d191462381628d407f2a5a08fa0620b9b12b45aecbbdb7aa0d20").0,
    )
    .unwrap();

    let compute_unit_id_1 =
        fixed_bytes!("ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");
    let compute_unit_id_2 =
        fixed_bytes!("bd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");
    let compute_unit_id_3 =
        fixed_bytes!("cd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");

    let compute_unit_1 = ComputeUnit {
        id: compute_unit_id_1,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
    };

    let compute_unit_2 = ComputeUnit {
        id: compute_unit_id_2,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
    };

    let compute_unit_3 = ComputeUnit {
        id: compute_unit_id_3,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
    };

    let compute_peer_1 = ComputePeer {
        offerId: offer_id,
        commitmentId: FixedBytes::<32>::ZERO,
        unitCount: U256::from(1),
        owner: owner_address,
    };

    test_entities
        .chain_server
        .register_peer(RegisterPeerParams {
            state: hashmap! {
               test_entities.swarms[0].peer_id => PeerState {
                    units: vec![compute_unit_1.clone(), compute_unit_2.clone(), compute_unit_3],
                    compute_peer: compute_peer_1
                },
            },
        })
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // Create commitment with status wait delegation
    test_entities
        .chain_server
        .create_commitment(test_entities.swarms[0].peer_id, &commitment_id)
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    let epoch = test_entities
        .chain_server
        .activate_commitment(test_entities.swarms[0].peer_id, &commitment_id, 100)
        .await;
    //active commitment on epoch 3
    test_entities.chain_server.move_to_epoch(epoch).await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    test_entities
        .chain_server
        .create_deal(CreateDealParams {
            deal_id: deal_address,
            peer_id: test_entities.swarms[0].peer_id,
            commitment_id,
            unit: compute_unit_1,
        })
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    test_entities
        .chain_server
        .change_deal_status(deal_address, Status::INSUFFICIENT_FUNDS)
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    //assert CCP
    let result_1 = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    let result_2 = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    assert_eq!(
        result_1.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result_2.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result_1.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );
    assert_eq!(
        result_2.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );

    assert_allocation(
        result_1.cu_allocation,
        vec![
            (
                PhysicalCoreId::new(125),
                "cd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
            ),
            (
                PhysicalCoreId::new(126),
                "bd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
            ),
            (
                PhysicalCoreId::new(127),
                "ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
            ),
        ],
    );

    assert_allocation(
        result_2.cu_allocation,
        vec![
            (
                PhysicalCoreId::new(125),
                "bd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
            ),
            (
                PhysicalCoreId::new(126),
                "cd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95".to_string(),
            ),
        ],
    );
}

fn assert_allocation(
    allocation: HashMap<PhysicalCoreId, OrHex<CUID>>,
    expected: Vec<(PhysicalCoreId, String)>,
) {
    let allocation = allocation
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .sorted_by(|(l, _), (r, _)| l.cmp(r));
    assert!(allocation.eq(expected))
}
