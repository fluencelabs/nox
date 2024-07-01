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
