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
