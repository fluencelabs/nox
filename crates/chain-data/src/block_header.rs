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

use alloy_primitives::{U256, U64};
use alloy_serde_macro::{U256_from_String, U64_from_String};
use serde::Deserialize;
use serde_json::Value;
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct BlockHeader {
    #[serde(deserialize_with = "U64_from_String")]
    pub number: U64,
    #[serde(deserialize_with = "U256_from_String")]
    pub timestamp: U256,
}

impl BlockHeader {
    pub fn from_json(json: Value) -> eyre::Result<Self> {
        serde_json::from_value(json.clone())
            .map_err(|err| eyre::eyre!("failed to parse header {err}; got {json}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_block_header_from_json() {
        let json = json!({"number":"0x11","timestamp":"0x11"});
        let header = BlockHeader::from_json(json).unwrap();
        assert_eq!(header.number, U64::from(17));
        assert_eq!(header.timestamp, U256::from(17));

        let json = json!({"number":"11","timestamp":"11"});
        let header = BlockHeader::from_json(json).unwrap();
        assert_eq!(header.number, U64::from(11));
        assert_eq!(header.timestamp, U256::from(11));

        let json = json!({"number":"0xa1","timestamp":"0xa1"});
        let header = BlockHeader::from_json(json).unwrap();
        assert_eq!(header.number, U64::from(161));
        assert_eq!(header.timestamp, U256::from(161));
    }
}
