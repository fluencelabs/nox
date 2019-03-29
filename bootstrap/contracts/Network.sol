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

pragma solidity ^0.5.7;

import "./Deployer.sol";

/*
 * This contract provides different getter methods for inspection of the Fluence network state.
 *
 * All information is stored in the Deployer and this contract just transforms it into tool-readable form.
 * Main consumers of this contract are Fluence command line utilities and web interfaces. So while it can be used by
 * humans directly, it's not designed for that purpose.
 *
 */
contract Network is Deployer {
    /** @dev Retrieves node's info
     * @param nodeID ID of node (Tendermint consensus key)
     * returns tuple representation of Node structure
     */
    function getNode(bytes32 nodeID)
    external
    view
    returns (bytes24, uint16, uint16, address, bool, uint256[] memory)
    {
        Node memory node = nodes[nodeID];
        return (
            node.nodeAddress,
            node.apiPort,
            node.capacity,
            node.owner,
            node.isPrivate,
            node.appIDs
        );
    }


    /** @dev Retrieves currently running apps for specified node's workers
     *  @param nodeID ID of node (Tendermint consensus key)
     *  returns IDs apps hosted by this node
     */
    function getNodeApps(bytes32 nodeID)
    external
    view
    returns (uint256[] memory)
    {
        return nodes[nodeID].appIDs;
    }

    /** @dev Retrieves assigned App and other cluster info by clusterID
     * @param appID unique id of cluster
     * returns tuple representation of a Cluster
     */
    function getApp(uint256 appID)
    external
    view
    returns (bytes32, bytes32, Storage, uint8, address, bytes32[] memory, uint, bytes32[] memory)
    {
        App memory app = apps[appID];
        require(app.appID > 0, "there is no such app");

        return (
            app.storageHash,
            app.storageReceipt,
            app.storageType,
            app.clusterSize,
            app.owner,
            app.pinToNodes,

            app.cluster.genesisTime,
            app.cluster.nodeIDs
        );
    }

    /** @dev Retrieves addresses and ports of cluster's workers
     * @param appID unique id of app
     */
    function getAppWorkers(uint256 appID)
    external
    view
    returns (bytes24[] memory, uint16[] memory)
    {
        App memory app = apps[appID];
        require(app.appID > 0, "there is no such app");

        bytes24[] memory addresses = new bytes24[](app.cluster.nodeIDs.length);
        uint16[] memory ports = new uint16[](app.cluster.nodeIDs.length);

        for (uint8 i = 0; i < app.cluster.nodeIDs.length; i++) {
            addresses[i] = nodes[app.cluster.nodeIDs[i]].nodeAddress;
            ports[i] = nodes[app.cluster.nodeIDs[i]].apiPort;
        }

        return (
            addresses,
            ports
        );
    }

    /** @dev Gets nodes and clusters IDs
     * return (node IDs, cluster IDs)
     */
    function getNodesIds()
    external
    view
    returns (bytes32[] memory)
    {
        return nodesIds;
    }

    function getAppIDs()
    external
    view
    returns (uint256[] memory)
    {
        return appIDs;
    }
}
