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

/*
 * This contract is a read model for the main Fluence (Deployer + Network) contract
 *
 * It's main purpose is to allow to change read API independently of the main Fluence contract.
 *
 */
contract Dashboard {
    Network network;

    constructor(address addr) public {
        network = Network(addr);
    }

    function getApps()
    external
    view
    returns (uint256[] memory, bytes32[] memory, address[] memory) {
        uint256[] memory appIDs = network.getAppIDs();

        bytes32[] memory storageHashes = new bytes32[](appIDs.length);
        address[] memory owners = new address[](appIDs.length);
        for (uint8 i = 0; i < appIDs.length; i++) {
            (bytes32 storageHash,,,,address owner,,,) = network.getApp(appIDs[i]);
            storageHashes[i] = storageHash;
            owners[i] = owner;
        }

        return (appIDs, storageHashes, owners);
    }
}
