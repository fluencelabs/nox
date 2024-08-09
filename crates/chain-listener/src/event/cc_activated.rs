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
sol! {
    /// @dev Emitted when a commitment is activated. Commitment can be activated only if delegator deposited collateral.
    /// @param peerId Peer id which linked to the commitment
    /// @param commitmentId Commitment id which activated
    /// @param startEpoch The start epoch of the commitment
    /// @param endEpoch The end epoch of the commitment
    /// @param unitIds Compute unit ids which linked to the commitment
    #[derive(Debug)]
    event CommitmentActivated(
        bytes32 indexed peerId,
        bytes32 indexed commitmentId,
        uint256 startEpoch,
        uint256 endEpoch,
        bytes32[] unitIds
    );
}

#[cfg(test)]
mod test {
    use super::CommitmentActivated;
    use alloy_primitives::Uint;
    use alloy_sol_types::{SolEvent, Word};
    use chain_data::parse_peer_id;
    use hex_utils::decode_hex;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_cc_activated_topic() {
        assert_eq!(
            CommitmentActivated::SIGNATURE_HASH.to_string(),
            "0x0b0a4688a90d1b24732d05ddf4925af69f02cd7d9a921b1cdcd4a7c2b6d57d68"
        );
    }

    #[tokio::test]
    async fn test_chain_parsing_ok() {
        let data = "0x000000000000000000000000000000000000000000000000000000000000007b00000000000000000000000000000000000000000000000000000000000001c800000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000001c04d94f1e85788b245471c87490f42149b09503fe3af46733e4b5adf94583105".to_string();
        let topics = vec![
            CommitmentActivated::SIGNATURE_HASH.to_string(),
            "0xc586dcbfc973643dc5f885bf1a38e054d2675b03fe283a5b7337d70dda9f7171".to_string(),
            "0x27e42c090aa007a4f2545547425aaa8ea3566e1f18560803ac48f8e98cb3b0c9".to_string(),
        ];
        let result = CommitmentActivated::decode_raw_log(
            topics.into_iter().map(|t| Word::from_str(&t).unwrap()),
            &decode_hex(&data).unwrap(),
            true,
        );

        assert!(result.is_ok(), "can't parse data: {:?}", result);
        let result = result.unwrap();
        assert_eq!(
            parse_peer_id(result.peerId.as_slice()).unwrap().to_string(),
            "12D3KooWP7RkvkBhbe7ATd451zxTifzF6Gm1uzCDadqQueET7EMe" // it's also the second topic
        );

        assert_eq!(
            result.commitmentId.to_string(),
            "0x27e42c090aa007a4f2545547425aaa8ea3566e1f18560803ac48f8e98cb3b0c9" // it's the third topic
        );
        assert_eq!(result.startEpoch, Uint::from(123));
        assert_eq!(result.endEpoch, Uint::from(456));

        assert_eq!(result.unitIds.len(), 1);
        assert_eq!(
            result.unitIds[0].to_string(),
            "0xc04d94f1e85788b245471c87490f42149b09503fe3af46733e4b5adf94583105"
        )
    }
}
