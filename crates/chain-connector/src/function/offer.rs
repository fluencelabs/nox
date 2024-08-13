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
            bytes32 onchainWorkerId;
        }

        /// @dev Returns the compute peer info
        function getComputePeer(bytes32 peerId) external view returns (ComputePeer memory);
        /// @dev Returns the compute units info of a peer
        function getComputeUnits(bytes32 peerId) external view returns (ComputeUnit[] memory);
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
    use alloy_primitives::{hex, Address, FixedBytes, U256};
    use alloy_sol_types::SolValue;
    use hex_utils::decode_hex;
    use std::str::FromStr;

    #[test]
    fn encode() {
        let cu = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5";
        let onchain_id = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d1";
        let onchain_id_2 = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d2";
        let cu_id = FixedBytes::from_slice(&decode_hex(cu).unwrap());

        let expected_deal_id_2 = "0x1111111111111111111111111111111111111111";
        let expected_cuid_2 = "ba3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d1";

        let cu1 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:1>64}", 1)).unwrap()),
        };
        let cu3 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:2>64}", 2)).unwrap()),
        };
        let cu4 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:3>64}", 3)).unwrap()),
        };

        let cu2 = super::ComputeUnit {
            id: FixedBytes::from_slice(&decode_hex(expected_cuid_2).unwrap()),
            deal: Default::default(),
            startEpoch: Default::default(),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(onchain_id_2).unwrap()),
        };
        let cu5 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:4>64}", 4)).unwrap()),
        };
        let cu6 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:5>64}", 5)).unwrap()),
        };

        let cu7 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:5>64}", 6)).unwrap()),
        };
        let cu8 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:5>64}", 6)).unwrap()),
        };
        let cu9 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:5>64}", 7)).unwrap()),
        };
        let cu10 = super::ComputeUnit {
            id: cu_id,
            deal: Address::from_str(expected_deal_id_2).unwrap(),
            startEpoch: U256::from(1),
            onchainWorkerId: FixedBytes::from_slice(&decode_hex(&format!("{:5>64}", 8)).unwrap()),
        };

        let cus: Vec<super::ComputeUnit> = vec![cu1, cu2, cu3, cu4, cu5, cu6, cu7, cu8, cu9, cu10];
        println!("{:?}", hex::encode(cus.abi_encode()));

        /*
        let s = "00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000002011111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99";
        let s = "00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000018000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000001ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000002ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b9900000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111ffa0611a099ab68ad7c3c67b4ca5bbbee7a58b99000000000000000000000000880a53a54785df22ba804aee81ce8bd0d45bdedc000000000000000000000000880a53a54785df22ba804aee81ce8bd0d45bdedc00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111880a53a54785df22ba804aee81ce8bd0d45bdedc000000000000000000000001880a53a54785df22ba804aee81ce8bd0d45bdedc000000000000000000000000880a53a54785df22ba804aee81ce8bd0d45bdedc00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111880a53a54785df22ba804aee81ce8bd0d45bdedc000000000000000000000002880a53a54785df22ba804aee81ce8bd0d45bdedc000000000000000000000000880a53a54785df22ba804aee81ce8bd0d45bdedc00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111880a53a54785df22ba804aee81ce8bd0d45bdedc00000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000000000000000000000000000000000000000fffbc11111111111111111111111167b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000167b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000000000000000000000000000000000000000fffbc11111111111111111111111167b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000267b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000067b2ad3866429282e16e55b715d12a77f85b7ce800000000000000000000000000000000000000000000000000000000000fffbc11111111111111111111111167b2ad3866429282e16e55b715d12a77f85b7ce80000000000000000000000001234563866429282e16e55b715d12a77f85b7cc90000000000000000000000001234563866429282e16e55b715d12a77f85b7cc900000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111111234563866429282e16e55b715d12a77f85b7cc90000000000000000000000011234563866429282e16e55b715d12a77f85b7cc90000000000000000000000001234563866429282e16e55b715d12a77f85b7cc900000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111111234563866429282e16e55b715d12a77f85b7cc90000000000000000000000021234563866429282e16e55b715d12a77f85b7cc90000000000000000000000001234563866429282e16e55b715d12a77f85b7cc900000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111111234563866429282e16e55b715d12a77f85b7cc9000000000000000000000000991b64a54785df22ba804aee81ce8bd0d45bdabb000000000000000000000000991b64a54785df22ba804aee81ce8bd0d45bdabb00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111991b64a54785df22ba804aee81ce8bd0d45bdabb000000000000000000000001991b64a54785df22ba804aee81ce8bd0d45bdabb000000000000000000000000991b64a54785df22ba804aee81ce8bd0d45bdabb00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111991b64a54785df22ba804aee81ce8bd0d45bdabb000000000000000000000002991b64a54785df22ba804aee81ce8bd0d45bdabb000000000000000000000000991b64a54785df22ba804aee81ce8bd0d45bdabb00000000000000000000000000000000000000000000000000000000000fffbc111111111111111111111111991b64a54785df22ba804aee81ce8bd0d45bdabb0000000000000000000000003665748409e712cd91b428c18e07a8e37b44c47e0000000000000000000000003665748409e712cd91b428c18e07a8e37b44c47e00000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111113665748409e712cd91b428c18e07a8e37b44c47e0000000000000000000000013665748409e712cd91b428c18e07a8e37b44c47e0000000000000000000000003665748409e712cd91b428c18e07a8e37b44c47e00000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111113665748409e712cd91b428c18e07a8e37b44c47e0000000000000000000000023665748409e712cd91b428c18e07a8e37b44c47e0000000000000000000000003665748409e712cd91b428c18e07a8e37b44c47e00000000000000000000000000000000000000000000000000000000000fffbc1111111111111111111111113665748409e712cd91b428c18e07a8e37b44c47e";

        let ss = hex::decode(s).unwrap();
        //let cus: Vec<super::ComputeUnit>
        let r: Result<Vec<super::ComputeUnit>, _> = SolValue::abi_decode(&ss, true);
        println!("{:?}", r);
        */
    }

    #[test]
    fn decode_compute_unit() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d50000000000000000000000005e3d0fde6f793b3115a9e7f5ebc195bbeed35d6c00000000000000000000000000000000000000000000000000000000000003e8bb3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633dd";
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
        assert_eq!(
            compute_unit.onchainWorkerId,
            hex!("bb3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633dd")
        )
    }

    #[test]
    fn decode_compute_unit_no_deal_no_worker() {
        let data = "aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003e80000000000000000000000000000000000000000000000000000000000000000";
        let compute_unit = super::ComputeUnit::abi_decode(&decode_hex(data).unwrap(), true);
        assert!(compute_unit.is_ok());
        let compute_unit = compute_unit.unwrap();
        assert_eq!(
            compute_unit.id,
            hex!("aa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5")
        );
        assert!(compute_unit.deal.is_zero());
        assert_eq!(compute_unit.startEpoch, U256::from(1000));
        assert!(compute_unit.onchainWorkerId.is_zero())
    }

    #[test]
    fn decode_compute_peer_no_commitment() {
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

    #[test]
    fn decode_compute_peer() {
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
