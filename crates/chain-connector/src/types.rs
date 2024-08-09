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
use crate::error::ConnectorError;
use crate::function::Deal;
use alloy_primitives::U256;
use ccp_shared::types::{Difficulty, GlobalNonce, CUID};
use chain_data::parse_peer_id;
use eyre::{eyre, Report};
use serde::{Deserialize, Serialize};
use types::DealId;

pub type Result<T> = std::result::Result<T, ConnectorError>;

#[derive(Debug, Serialize, Deserialize)]
pub struct DealResult {
    pub success: bool,
    pub error: Vec<String>,
    pub deal_id: DealId,
    pub deal_info: Vec<DealInfo>,
}

impl DealResult {
    pub fn with_error(deal_id: DealId, err: String) -> Self {
        Self {
            success: false,
            error: vec![err],
            deal_id,
            deal_info: vec![],
        }
    }

    pub fn new(deal_id: DealId, info: DealInfo) -> Self {
        Self {
            success: true,
            error: vec![],
            deal_id,
            deal_info: vec![info],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DealInfo {
    pub status: Deal::Status,
    pub unit_ids: Vec<CUID>,
    pub app_cid: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxReceiptResult {
    pub success: bool,
    pub error: Vec<String>,
    pub status: String,
    pub receipt: Vec<TxReceiptInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxReceiptInfo {
    pub block_number: String,
    pub tx_hash: String,
}

impl TxReceiptResult {
    pub fn pending() -> Self {
        Self {
            success: true,
            error: vec![],
            status: "pending".to_string(),
            receipt: vec![],
        }
    }

    pub fn processed(receipt: TxReceipt) -> Self {
        Self {
            success: true,
            error: vec![],
            status: if receipt.is_ok { "ok" } else { "failed" }.to_string(),
            receipt: vec![TxReceiptInfo {
                block_number: receipt.block_number,
                tx_hash: receipt.transaction_hash,
            }],
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            error: vec![msg],
            status: "".to_string(),
            receipt: vec![],
        }
    }
}

#[derive(Debug)]
pub struct TxReceipt {
    pub is_ok: bool,
    pub transaction_hash: String,
    pub block_number: String,
}

#[derive(Debug)]
pub enum TxStatus {
    Pending,
    Processed(TxReceipt),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTxReceipt {
    status: String,
    transaction_hash: String,
    block_number: String,
}

impl RawTxReceipt {
    pub fn to_tx_receipt(self) -> TxReceipt {
        TxReceipt {
            // if status is "0x1" transaction was successful
            is_ok: self.status == "0x1",
            transaction_hash: self.transaction_hash,
            block_number: self.block_number,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SubnetWorker {
    pub cu_ids: Vec<String>,
    pub host_id: String,
    pub worker_id: Vec<String>,
}

impl TryFrom<Deal::Worker> for SubnetWorker {
    type Error = Report;
    fn try_from(deal_worker: Deal::Worker) -> eyre::Result<Self> {
        let mut worker_id = vec![];
        if !deal_worker.offchainId.is_zero() {
            let w_id = parse_peer_id(&deal_worker.offchainId.0)
                .map_err(|err| eyre!("Failed to parse worker.offchainId: {err}"))?
                .to_base58();
            worker_id.push(w_id)
        }
        let cu_ids = deal_worker
            .computeUnitIds
            .into_iter()
            .map(|id| id.to_string())
            .collect();
        let peer_id = parse_peer_id(&deal_worker.peerId.0)
            .map_err(|err| eyre!("Failed to parse worker.peerId: {err}"))?;

        Ok(Self {
            cu_ids,
            host_id: peer_id.to_base58(),
            worker_id,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SubnetResolveResult {
    pub success: bool,
    pub workers: Vec<SubnetWorker>,
    pub error: Vec<String>,
}

pub struct CCInitParams {
    pub difficulty: Difficulty,
    pub init_timestamp: U256,
    pub current_timestamp: U256,
    pub global_nonce: GlobalNonce,
    pub current_epoch: U256,
    pub epoch_duration: U256,
    pub min_proofs_per_epoch: U256,
    pub max_proofs_per_epoch: U256,
}
