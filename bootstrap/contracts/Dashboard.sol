/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

pragma solidity ^0.5.8;

import "./Network.sol";

contract Dashboard {
    Network network;

    constructor(address addr) public {
        network = Network(addr);
    }

    function getApps()
    external
    view
    returns (uint256[] memory, bytes32[] memory) {
        uint256[] memory appIDs = network.getAppIDs();

        bytes32[] memory storageHashes = new bytes32[](appIDs.length);
        for (uint8 i = 0; i < appIDs.length; i++) {
            (bytes32 storageHash,,,,,,,) = network.getApp(appIDs[i]);
            storageHashes[i] = storageHash;
        }

        return (appIDs, storageHashes);
    }
}