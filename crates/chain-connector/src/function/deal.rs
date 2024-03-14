use alloy_sol_types::sol;
sol! {
    contract Deal {
        #[derive(Debug)]
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
    }
}
