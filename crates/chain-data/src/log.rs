use libp2p_identity::ParseError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::chain_data::EventField::{Indexed, NotIndexed};
use crate::chain_data::{parse_chain_data, ChainData, ChainEvent, EventField};
use crate::error::ChainDataError;
use crate::log::LogParseError::{MissingToken, MissingTopic};

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
    EthError(#[from] ethabi::Error),
    #[error(transparent)]
    DecodeHex(#[from] hex::FromHexError),
    #[error("parsed data doesn't correspond to the expected signature: {0:?}")]
    SignatureMismatch(Vec<EventField>),
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

/// Parse Event Log to specified DTO
///
/// Logs consist of data fields, much like ADT. Fields can indexed and not indexed.
///
/// Data for indexed fields is encoded in 'log.topics', starting from 1th topic, i.e. 0th is skipped
/// Data for non indexed fields is encoded in 'log.data'.
///
/// Indexed and non indexed data fields can be interleaved.
/// That forces a certain parsing scheme, which is implemented below.
pub fn parse_log<U: ChainData, T: ChainEvent<U>>(log: Log) -> Result<T, LogParseError> {
    log::debug!("Parse log from block {:?}", log.block_number);
    let result: Result<_, LogParseError> = try {
        // event log signature, i.e. data field types
        let signature = U::signature();
        // gather data types for non indexed ("indexless") fields
        let indexless = signature
            .clone()
            .into_iter()
            .filter_map(|t| match t {
                NotIndexed(t) => Some(t),
                Indexed(_) => None,
            })
            .collect::<Vec<_>>();
        // parse all non indexed fields to tokens
        let indexless = parse_chain_data(&log.data, &indexless)?;

        // iterate through data field types (signature), and take
        // data `Token` from either 'indexless' or 'topics'
        let mut indexless = indexless.into_iter();
        // skip first topic, because it contains actual topic, and not indexed data field
        let mut topics = log.topics.into_iter().skip(1);
        // accumulate tokens here
        let mut tokens = vec![];
        for (position, event_field) in signature.into_iter().enumerate() {
            match event_field {
                NotIndexed(_) => {
                    // take next token for non indexed data field
                    let token = indexless.next().ok_or(MissingToken {
                        position,
                        event_field,
                    })?;
                    tokens.push(token);
                }
                ef @ Indexed(_) => {
                    let topic = topics.next().ok_or(MissingTopic {
                        position,
                        event_field: ef.clone(),
                    })?;
                    // parse indexed field to token one by one
                    let parsed = parse_chain_data(&topic, &[ef.clone().param_type()])?;
                    debug_assert!(parsed.len() == 1, "parse of an indexed event fields yielded several tokens, expected a single one");
                    let token = parsed.into_iter().next().ok_or(MissingToken {
                        position,
                        event_field: ef,
                    })?;
                    tokens.push(token)
                }
            }
        }

        if tokens.is_empty() {
            return Err(LogParseError::NoTokens);
        }

        let block_number = log.block_number.clone();
        let log = U::parse(&mut tokens.into_iter())?;
        T::new(block_number, log)
    };

    if let Err(e) = result.as_ref() {
        log::warn!(target: "connector",
            "Cannot parse deal log from block {}: {:?}",
            log.block_number,
            e.to_string()
        );
    }

    result
}
