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

use crate::error::ChainDataError;
use ethabi::{ParamType, Token};
use hex_utils::decode_hex;

#[derive(Debug, Clone, PartialEq)]
/// Kind of the field in Chain Event
pub enum EventField {
    /// If field is indexed, it's passed among topics
    Indexed(ParamType),
    /// If field is not indexed, it's passed in log.data
    NotIndexed(ParamType),
}

impl EventField {
    pub fn param_type(self) -> ParamType {
        match self {
            EventField::Indexed(t) => t,
            EventField::NotIndexed(t) => t,
        }
    }
}

pub trait ChainData {
    fn event_name() -> &'static str;
    fn signature() -> Vec<EventField>;
    fn parse(data_tokens: &mut impl Iterator<Item = Token>) -> Result<Self, ChainDataError>
    where
        Self: Sized;

    fn topic() -> String {
        let sig: Vec<_> = Self::signature()
            .into_iter()
            .map(|t| t.param_type())
            .collect();
        let hash = ethabi::long_signature(Self::event_name(), &sig);
        format!("0x{}", hex::encode(hash.as_bytes()))
    }
}

pub trait ChainEvent<ChainData> {
    fn new(block_number: String, data: ChainData) -> Self;
}

/// Parse data from chain. Accepts data with and without "0x" prefix.
pub fn parse_chain_data(data: &str, signature: &[ParamType]) -> Result<Vec<Token>, ChainDataError> {
    if data.is_empty() {
        return Err(ChainDataError::Empty);
    }
    let data = decode_hex(data).map_err(ChainDataError::DecodeHex)?;
    Ok(ethabi::decode(signature, &data)?)
}
