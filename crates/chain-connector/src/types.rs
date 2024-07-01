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

use serde::{Deserialize, Serialize};

use types::DealId;

use crate::error::ConnectorError;
use crate::function::Deal;

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
    pub unit_ids: Vec<Vec<u8>>,
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
