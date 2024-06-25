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
