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

use alloy_sol_types::sol;
use chain_connector::PendingUnit;

use core_distributor::CUID;

sol! {
    /// @dev Emitted when a unit activated.
    /// Unit is activated when it returned from deal
    /// @param commitmentId Commitment id
    /// @param unitId Compute unit id which activated
    #[derive(Debug)]
    event UnitActivated(
        bytes32 indexed commitmentId,
        bytes32 indexed unitId,
        uint256 startEpoch
    );
}

impl From<UnitActivated> for PendingUnit {
    fn from(data: UnitActivated) -> Self {
        PendingUnit {
            id: CUID::new(data.unitId.0),
            start_epoch: data.startEpoch,
        }
    }
}
#[cfg(test)]
mod test {
    use super::UnitActivated;
    use alloy_primitives::Uint;
    use alloy_sol_types::{SolEvent, Word};
    use std::str::FromStr;

    use hex_utils::decode_hex;

    #[tokio::test]
    async fn test_unit_activated_topic() {
        assert_eq!(
            UnitActivated::SIGNATURE_HASH.to_string(),
            "0x8e4b27eeb3194deef0b3140997e6b82f53eb7350daceb9355268009b92f70add"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x000000000000000000000000000000000000000000000000000000000000007b".to_string();
        let topics = vec![
            UnitActivated::SIGNATURE_HASH.to_string(),
            "0x431688393bc518ef01e11420af290b92f3668dca24fc171eeb11dd15bcefad72".to_string(),
            "0xd33bc101f018e42351fbe2adc8682770d164e27e2e4c6454e0faaf5b8b63b90e".to_string(),
        ];

        let result = UnitActivated::decode_raw_log(
            topics.into_iter().map(|t| Word::from_str(&t).unwrap()),
            &decode_hex(&data).unwrap(),
            true,
        );

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap();
        assert_eq!(
            result.commitmentId.to_string(),
            "0x431688393bc518ef01e11420af290b92f3668dca24fc171eeb11dd15bcefad72" // it's the second topic
        );
        assert_eq!(
            result.unitId.to_string(),
            "0xd33bc101f018e42351fbe2adc8682770d164e27e2e4c6454e0faaf5b8b63b90e" // it's also the third topic
        );

        assert_eq!(result.startEpoch, Uint::from(123));
    }
}
