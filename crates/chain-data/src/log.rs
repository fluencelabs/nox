/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use alloy_sol_types::{SolEvent, Word};
use hex_utils::decode_hex;
use libp2p_identity::ParseError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

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
    DecodeHex(#[from] hex::FromHexError),
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
