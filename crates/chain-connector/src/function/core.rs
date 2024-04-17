use alloy_sol_types::sol;
sol! {
    contract Core {
        function capacity() external view returns (address);

        function market() external view returns (address);

        /// @dev Returns current epoch
        /// @return current epoch number
        function currentEpoch() external view returns (uint256);

        /// @dev Returns epoch duration
        /// @return epochDuration in seconds
        function epochDuration() external view returns (uint256);

        /// @dev Returns epoch init timestamp
        /// @return initTimestamp in seconds
        function initTimestamp() external view returns (uint256);

        /// @dev Returns the difficulty for CCP
        function difficulty() external view returns (bytes32);

        /// @dev Returns the min required randomX proofs per epoch for the 1 CU.
        /// @dev  If lower than this - CU is failed and CC slashed.
        function minProofsPerEpoch() external view returns (uint256);

        /// @dev Returns the max randomX proofs per epoch
        function maxProofsPerEpoch() external view returns (uint256);
    }
}
