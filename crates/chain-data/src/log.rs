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

use std::str::FromStr;

use alloy_sol_types::{SolEvent, Word};
use libp2p_identity::ParseError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use hex_utils::{decode_hex, FromHexError};

use crate::chain_data::EventField;
use crate::error::ChainDataError;
use crate::LogParseError::EthError;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    // Log arguments
    pub data: String,
    // The block number in hex (with 0x prefix) that contains this log
    pub block_number: String,
    // true when the log was removed, due to a chain reorganization. false if its a valid log.
    #[serde(default)]
    pub removed: bool,
    pub topics: Vec<String>,
}

#[derive(Debug, Error)]
pub enum LogParseError {
    #[error(transparent)]
    EthError(#[from] alloy_sol_types::Error),
    #[error(transparent)]
    DecodeHex(#[from] FromHexError),
    #[error(transparent)]
    DecodeConstHex(#[from] const_hex::FromHexError),
    #[error(
        "incorrect log signature: not found token for field #{position} of type ${event_field:?}"
    )]
    MissingToken {
        position: usize,
        event_field: EventField,
    },
    #[error("incorrect log signature: not found topic for indexed field #{position} of type ${event_field:?}")]
    MissingTopic {
        position: usize,
        event_field: EventField,
    },
    #[error("missing token for field '{0}'")]
    MissingParsedToken(&'static str),
    #[error("invalid token for field '{0}'")]
    InvalidParsedToken(&'static str),
    #[error("invalid compute peer id: '{0}'")]
    InvalidComputePeerId(#[from] ParseError),
    #[error(transparent)]
    ChainData(#[from] ChainDataError),
    #[error("no tokens after deserialization")]
    NoTokens,
}

pub fn parse_log<T: SolEvent>(log: Log) -> Result<T, LogParseError> {
    let mut topics = vec![];
    for t in log.topics {
        topics.push(Word::from_str(&t)?);
    }
    SolEvent::decode_raw_log(topics.iter(), &decode_hex(&log.data)?, true).map_err(EthError)
}
