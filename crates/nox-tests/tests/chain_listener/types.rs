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

use crate::chain_listener::r#const::{DEFAULT_DIFFICULTY, DEFAULT_GLOBAL_NONCE};
use alloy_primitives::{Address, FixedBytes, U256};
use ccp_shared::types::{Difficulty, GlobalNonce, PhysicalCoreId, CUID};
use chain_connector::Deal::Status;
use chain_connector::Offer::{ComputePeer, ComputeUnit};
use chain_connector::{CCStatus, CommitmentId};
use fluence_libp2p::PeerId;
use hex::FromHex;
use jsonrpsee::SubscriptionMessage;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ChainState {
    pub(crate) block_number: U256,
    pub(crate) block_duration: U256,
    pub(crate) difficulty: Difficulty,
    pub(crate) init_timestamp: U256,
    pub(crate) global_nonce: GlobalNonce,
    pub(crate) current_epoch: U256,
    pub(crate) epoch_duration: U256,
    pub(crate) min_proofs_per_epoch: U256,
    pub(crate) max_proofs_per_epoch: U256,
    pub(crate) commitment_statuses: HashMap<CommitmentId, CCStatus>,
    pub(crate) commitment_activation_at: HashMap<CommitmentId, U256>,
    pub(crate) peer_states: HashMap<PeerId, PeerState>,
    pub(crate) deal_statuses: HashMap<Address, Status>,
    pub(crate) unit_state: HashMap<FixedBytes<32>, UnitState>,
}

impl Default for ChainState {
    fn default() -> Self {
        Self {
            block_number: U256::from(0),
            block_duration: U256::from(100),
            difficulty: Difficulty::from_hex(DEFAULT_DIFFICULTY).unwrap(),
            init_timestamp: U256::from(0),
            global_nonce: GlobalNonce::from_hex(DEFAULT_GLOBAL_NONCE).unwrap(),
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

pub struct Context {
    pub(crate) new_heads_sender: tokio::sync::broadcast::Sender<SubscriptionMessage>,
    pub(crate) logs_sender: tokio::sync::broadcast::Sender<LogsParams>,
    pub(crate) chain_state: Arc<Mutex<ChainState>>,
}

#[derive(Clone, Debug)]
pub struct LogsParams {
    pub(crate) address: String,
    pub(crate) topics: Vec<String>,
    pub(crate) message: SubscriptionMessage,
}

pub struct PeerState {
    pub(crate) units: Vec<ComputeUnit>,
    pub(crate) compute_peer: ComputePeer,
}

pub struct UnitState {
    pub(crate) peer_id: PeerId,
    pub(crate) commitment_id: Option<CommitmentId>,
}

#[derive(Clone, Debug)]
pub struct ActiveCommitmentParams {
    pub(crate) global_nonce: GlobalNonce,
    pub(crate) difficulty: Difficulty,
    pub(crate) cu_allocation: HashMap<PhysicalCoreId, CUID>,
}

pub struct RegisterPeerParams {
    pub(crate) state: HashMap<PeerId, PeerState>,
}

pub struct CreateDealParams {
    pub(crate) deal_id: Address,
    pub(crate) peer_id: PeerId,
    pub(crate) commitment_id: CommitmentId,
    pub(crate) unit: ComputeUnit,
}
