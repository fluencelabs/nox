use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
sol! {
    contract Deal {
        struct CIDV1 {
            bytes4 prefixes;
            bytes32 hash;
        }

        #[derive(Debug, Serialize, Deserialize)]
        enum Status {
            // the deal does have enough funds to pay for the workers
            INSUFFICIENT_FUNDS,
            ACTIVE,
            // the deal is stopped
            ENDED,
            // the deal has a balance and waiting for workers
            NOT_ENOUGH_WORKERS,
            // the deal has balance less than the minimal balance. Min balance: 2 * targetWorkers * pricePerWorkerEpoch
            SMALL_BALANCE
        }

        /// @dev Returns the status of the deal
        function getStatus() external view returns (Status);

        /// @dev Returns the app CID
        function appCID() external view returns (CIDV1 memory);

        /// @dev Set worker ID for a compute unit. Compute unit can have only one worker ID
        function setWorker(bytes32 computeUnitId, bytes32 workerId) external;
    }
}
