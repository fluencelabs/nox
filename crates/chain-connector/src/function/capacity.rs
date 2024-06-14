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

use alloy_sol_types::{sol, SolError};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

sol! {
    #[derive(PartialEq, Debug)]
    enum CCStatus {
        Inactive,
        Active,
        // WaitDelegation - before collateral is deposited.
        WaitDelegation,
        // Status is WaitStart - means collateral deposited, and epoch should be proceed before Active.
        WaitStart,
        Failed,
        Removed
    }


    contract Capacity {

        /// @dev Throws if peer sent too many proofs for the commitment by unit per epoch
        error TooManyProofs();

        /// @dev Capacity commitment is not active
        error CapacityCommitmentIsNotActive(CCStatus status);

        function getGlobalNonce() external view returns (bytes32);


        /// @dev Returns the commitment status
        /// @param commitmentId Commitment id
        /// @return status commitment status
        function getStatus(bytes32 commitmentId) external view returns (CCStatus);

       /// @dev Submits a proof for the commitment
       /// @param unitId Compute unit id which provied the proof
       /// @param localUnitNonce The local nonce of the unit for calculating the target hash. It's the proof
       /// @param resultHash The target hash of this proof
        function submitProof(bytes32 unitId, bytes32 localUnitNonce, bytes32 resultHash) external;

        /// @dev Submits proofs for the commitment
        /// @param unitIds Compute unit ids which provide the proof
        /// @param localUnitNonces Local nonces of the units for calculating the target hashes. It's the proof
        /// @param resultHashes Target hashes of this proof
        function submitProofs(
            bytes32[] memory unitIds,
            bytes32[] memory localUnitNonces,
            bytes32[] memory resultHashes
        ) external;
    }
}

pub fn is_too_many_proofs(data: &str) -> bool {
    data.contains(&hex::encode(Capacity::TooManyProofs::SELECTOR))
}

pub fn is_commitment_not_active(data: &str) -> bool {
    data.contains(&hex::encode(
        Capacity::CapacityCommitmentIsNotActive::SELECTOR,
    ))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentId(pub [u8; 32]);

impl CommitmentId {
    pub fn new(data: [u8; 32]) -> Option<Self> {
        if data.iter().all(|&x| x == 0) {
            None
        } else {
            Some(Self(data))
        }
    }
}

impl Display for CommitmentId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
#[cfg(test)]
mod tests {

    use crate::function::capacity::CCStatus;
    use alloy_sol_types::SolType;

    #[tokio::test]
    async fn decode_commitment_status() {
        let data = "0000000000000000000000000000000000000000000000000000000000000001";
        let status: CCStatus = CCStatus::abi_decode(&hex::decode(data).unwrap(), true).unwrap();

        assert_eq!(format!("{:?}", status), "Active");
        assert_eq!(status, CCStatus::Active);
    }

    #[tokio::test]
    async fn decode_commitment_status_removed() {
        let data = "0000000000000000000000000000000000000000000000000000000000000005";
        let status = CCStatus::abi_decode(&hex::decode(data).unwrap(), true).unwrap();

        assert_eq!(status, super::CCStatus::Removed);
    }
}
