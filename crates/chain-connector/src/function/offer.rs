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

use crate::Offer::ComputeUnit;
use alloy_primitives::U256;
use alloy_sol_types::sol;
use ccp_shared::types::CUID;

sol! {
    contract Offer {
        struct ComputePeer {
            bytes32 offerId;
            bytes32 commitmentId;
            uint256 unitCount;
            address owner;
        }

        #[derive(Debug)]
        struct ComputeUnit {
            bytes32 id;
            address deal;
            uint256 startEpoch;
        }

        /// @dev Returns the compute peer info
        function getComputePeer(bytes32 peerId) external view returns (ComputePeer memory);
        /// @dev Returns the compute units info of a peer
        function getComputeUnits(bytes32 peerId) external view returns (ComputeUnit[] memory);

        /// @dev Return the compute unit from a deal
        function returnComputeUnitFromDeal(bytes32 unitId) external;
    }
}

/// "Peer doesn't exists" in Market.sol
pub const PEER_NOT_EXISTS: &str = "08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000125065657220646f65736e27742065786973740000000000000000000000000000";

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PendingUnit {
    pub id: CUID,
    pub start_epoch: U256,
}

impl PendingUnit {
    pub fn new(id: CUID, start_epoch: U256) -> Self {
        Self { id, start_epoch }
    }
}
impl From<ComputeUnit> for PendingUnit {
    fn from(unit: ComputeUnit) -> Self {
        Self {
            id: CUID::new(unit.id.0),
            start_epoch: unit.startEpoch,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Offer::ComputePeer;
    use alloy_primitives::{hex, U256};
    use alloy_sol_types::SolType;
    use hex_utils::decode_hex;

    #[tokio::test]
    async fn decode_compute_unit() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d50000000000000000000000005e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c00000000000000000000000000000000000000000000000000000000000003e8";
        let compute_unit = super::ComputeUnit::abi_decode(&decode_hex(data).unwrap(), true);
        assert!(compute_unit.is_ok());
        let compute_unit = compute_unit.unwrap();

        assert_eq!(
            compute_unit.id,
            hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
        );
        assert!(!compute_unit.deal.is_zero());
        assert_eq!(
            compute_unit.deal.to_string().to_lowercase(),
            "0x5e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c"
        );
        assert_eq!(compute_unit.startEpoch, U256::from(1000));
    }

    #[tokio::test]
    async fn decode_compute_unit_no_deal() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003e8";
        let compute_unit = super::ComputeUnit::abi_decode(&decode_hex(data).unwrap(), true);
        assert!(compute_unit.is_ok());
        let compute_unit = compute_unit.unwrap();
        assert_eq!(
            compute_unit.id,
            hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
        );
        assert!(compute_unit.deal.is_zero());
        assert_eq!(compute_unit.startEpoch, U256::from(1000));
    }

    #[tokio::test]
    async fn decode_compute_peer_no_commitment() {
        let data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let compute_peer = ComputePeer::abi_decode(&decode_hex(data).unwrap(), true);
        assert!(compute_peer.is_ok());
        let compute_peer = compute_peer.unwrap();
        assert_eq!(
            hex::encode(compute_peer.offerId),
            "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5"
        );
        assert!(compute_peer.commitmentId.is_zero());
        assert_eq!(compute_peer.unitCount, U256::from(2));
        assert_eq!(
            compute_peer.owner.to_string().to_lowercase(),
            "0x5b73c5498c1e3b4dba84de0f1833c4a029d90519"
        );
    }

    #[tokio::test]
    async fn decode_compute_peer() {
        let data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let compute_peer = ComputePeer::abi_decode(&decode_hex(data).unwrap(), true);
        assert!(compute_peer.is_ok());
        let compute_peer = compute_peer.unwrap();
        assert_eq!(
            hex::encode(compute_peer.offerId),
            "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5"
        );
        assert!(!compute_peer.commitmentId.is_zero());
        assert_eq!(
            hex::encode(compute_peer.commitmentId.0),
            "aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5"
        );
        assert_eq!(compute_peer.unitCount, U256::from(10));
        assert_eq!(
            compute_peer.owner.to_string().to_lowercase(),
            "0x5b73c5498c1e3b4dba84de0f1833c4a029d90519"
        );
    }
}
