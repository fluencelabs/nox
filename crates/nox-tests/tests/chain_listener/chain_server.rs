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
use crate::chain_listener::types::{
    ChainState, Context, CreateDealParams, LogsParams, RegisterPeerParams, UnitState,
};
use alloy_primitives::{Address, FixedBytes, U256};
use alloy_sol_types::sol_data::Array;
use alloy_sol_types::{SolCall, SolEvent, SolInterface, SolType, SolValue};
use chain_connector::Capacity::CapacityCalls;
use chain_connector::Core::CoreCalls;
use chain_connector::Deal::{DealCalls, Status};
use chain_connector::Offer::{ComputeUnit, OfferCalls};
use chain_connector::{CCStatus, CommitmentId, Offer};
use chain_data::{parse_peer_id, peer_id_to_bytes};
use chain_listener::{CommitmentActivated, ComputeUnitMatched, UnitDeactivated, CIDV1};
use clarity::Transaction;
use eyre::{eyre, OptionExt};
use fluence_libp2p::PeerId;
use futures::StreamExt;
use hex_utils::encode_hex_0x;
use jsonrpsee::server::ServerHandle;
use jsonrpsee::types::ErrorObject;
use jsonrpsee::{RpcModule, SubscriptionMessage};
use parking_lot::{Mutex, MutexGuard};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;

pub struct ChainServer {
    pub(crate) address: String,
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
        module.register_method("eth_estimateGas", |_params, _ctx, _ext| {
            Ok::<String, ErrorObject>(encode_hex_0x(U256::from(200).abi_encode()))
        })?;
        module.register_method("eth_maxPriorityFeePerGas", |_params, _ctx, _ext| {
            Ok::<String, ErrorObject>(encode_hex_0x(U256::from(200).abi_encode()))
        })?;
        module.register_method("eth_getTransactionCount", |_params, _ctx, _ext| {
            Ok::<String, ErrorObject>(encode_hex_0x(U256::from(1).abi_encode()))
        })?;
        module.register_method("eth_getBlockByNumber", |_params, ctx, _ext| {
            let state = ctx.chain_state.lock();
            Ok::<Value, ErrorObject>(json!({
                "number": state.block_number,
                "baseFeePerGas": U256::from(200),
                "timestamp": U256::from(10000)
            }))
        })?;
        let inner_market_contract_address = market_contract_address.clone();
        module.register_method("eth_sendRawTransaction", move |params, ctx, _ext| {
            let params = params.one::<String>();
            let data = params.map_err(|_| ErrorObject::owned(500, "", None::<String>))?;

            let transaction = hex::decode(data.trim_start_matches("0x"))
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
                                .map_err(|_| ErrorObject::owned(500, "expected ABI for function returnComputeUnitFromDealCall", None::<String>))?;

                        let state = ctx.chain_state.lock();
                        let unit_state = state
                            .unit_state
                            .get(&call.unitId)
                            .ok_or(ErrorObject::owned(500, format!("no such unitId {}", call.unitId), None::<String>))?;

                        let commitment_id = unit_state
                            .commitment_id
                            .clone()
                            .ok_or(ErrorObject::owned(500, &format!("compute unit {} doesn't have commitment id", call.unitId), None::<String>))?;

                        let event = CommitmentActivated {
                            peerId: FixedBytes::new(peer_id_to_bytes(unit_state.peer_id)),
                            commitmentId: FixedBytes::new(commitment_id.0),
                            startEpoch: state.current_epoch + U256::from(1),
                            endEpoch: state.current_epoch + U256::from(100),
                            unitIds: vec![call.unitId],
                        };

                        let data = event.encode_data();
                        let data = encode_hex_0x(data);

                        let message = SubscriptionMessage::from_json(&json!({
                          "topics": vec![
                            CommitmentActivated::SIGNATURE_HASH.to_string(),
                            encode_hex_0x(peer_id_to_bytes(unit_state.peer_id)),
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
                                encode_hex_0x(peer_id_to_bytes(unit_state.peer_id)),
                            ],
                            message,
                        };

                        ctx.logs_sender.send(message).unwrap();
                    } else {
                        return Err(ErrorObject::owned(500, &format!("Only TXs with returnComputeUnitFromDealCall sent to market address {} are supported", inner_market_contract_address), None::<String>));
                    }
                }
            }
            Ok::<String, ErrorObject>("done".to_string())
        })?;

        let inner_cc_contract_address = cc_contract_address.clone();
        let inner_market_contract_address = market_contract_address.clone();

        module.register_method("eth_call", move |params, ctx, _ext| {
            let params: eyre::Result<(String, Vec<u8>)> = try {
                let params = params.parse::<Value>()?;
                let array = params.as_array().ok_or_eyre("Not array")?;
                let head = array.first().ok_or_eyre("Empty array")?;
                let object = head.as_object().ok_or_eyre("Not object")?;
                let to = object.get("to").ok_or_eyre("To not found")?;
                let data = object.get("data").ok_or_eyre("Data not found")?;
                let to = to.as_str().ok_or_eyre("Not string")?;
                let data = data.as_str().ok_or_eyre("Not string")?;
                let data = hex::decode(data.trim_start_matches("0x"))?;
                (to.to_string(), data)
            };
            let (to, data) =
                params.map_err(|err| ErrorObject::owned(500, err.to_string(), None::<String>))?;
            let state = ctx.chain_state.lock();
            let deal_addresses = state
                .deal_statuses
                .keys()
                .map(|addr| addr.to_checksum(None))
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
                            Ok(encode_hex_0x(state.current_epoch.abi_encode()))
                        }
                        CoreCalls::epochDuration(_) => {
                            Ok(encode_hex_0x(state.epoch_duration.abi_encode()))
                        }
                        CoreCalls::initTimestamp(_) => {
                            Ok(encode_hex_0x(state.init_timestamp.abi_encode()))
                        }
                        CoreCalls::difficulty(_) => Ok(state.difficulty.to_string()),
                        CoreCalls::minProofsPerEpoch(_) => {
                            Ok(encode_hex_0x(state.min_proofs_per_epoch.abi_encode()))
                        }
                        CoreCalls::maxProofsPerEpoch(_) => {
                            Ok(encode_hex_0x(state.max_proofs_per_epoch.abi_encode()))
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
                            let status = state.commitment_statuses.get(&commitment_id).ok_or(
                                ErrorObject::owned(
                                    500,
                                    format!("commitment {commitment_id} not found"),
                                    None::<String>,
                                ),
                            )?;

                            let data = &status.abi_encode();
                            Ok(encode_hex_0x(data))
                        }
                        CapacityCalls::submitProof(_) => {
                            todo!()
                        }
                        CapacityCalls::submitProofs(_) => {
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
                            let state =
                                state.peer_states.get(&peer_id).ok_or(ErrorObject::owned(
                                    33,
                                    format!("Peer {peer_id} doesn't exists"),
                                    None::<String>,
                                ))?;
                            let data = &state.compute_peer.abi_encode();
                            Ok(encode_hex_0x(data))
                        }
                        OfferCalls::getComputeUnits(params) => {
                            let peer_id = parse_peer_id(params.peerId.to_vec())
                                .map_err(|_| ErrorObject::owned(500, "", None::<String>))?;
                            let state = state
                                .peer_states
                                .get(&peer_id)
                                .ok_or(ErrorObject::owned(500, "", None::<String>))?;
                            let data = Array::<ComputeUnit>::abi_encode(&state.units);
                            Ok(encode_hex_0x(data))
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
                            Ok(encode_hex_0x(data))
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
            |params, pending, ctx, _ext| async move {
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
        let data = encode_hex_0x(data);

        let message = SubscriptionMessage::from_json(&json!({
          "topics": vec![
            CommitmentActivated::SIGNATURE_HASH.to_string(),
            encode_hex_0x(peer_id_to_bytes(peer_id)),
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
                encode_hex_0x(peer_id_to_bytes(peer_id)),
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
            let data = encode_hex_0x(data);

            let message = SubscriptionMessage::from_json(&json!({
              "topics": vec![
                    UnitDeactivated::SIGNATURE_HASH.to_string(),
                    params.commitment_id.to_string(),
                    encode_hex_0x(params.unit.id.0)
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
            let data = encode_hex_0x(data);
            let message = SubscriptionMessage::from_json(&json!({
              "topics": vec![
                    ComputeUnitMatched::SIGNATURE_HASH.to_string(),
                    encode_hex_0x(peer_id_to_bytes(params.peer_id)),
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
                    encode_hex_0x(peer_id_to_bytes(params.peer_id)),
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
