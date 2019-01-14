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

pragma solidity ^0.4.24;

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
    returns (bytes24, uint16, uint16, address, bool, bytes32[])
    {
        Node memory node = nodes[nodeID];
        return (
            node.nodeAddress,
            node.nextPort,
            node.lastPort,
            node.owner,
            node.isPrivate,
            node.clusters
        );
    }


    /** @dev Retrieves currently running clusters for specified node's workers
     *  @param nodeID ID of node (Tendermint consensus key)
     *  returns IDs of clusters where the node is a member.
     */
    function getNodeClusters(bytes32 nodeID)
        external
        view
    returns (bytes32[])
    {
        return nodes[nodeID].clusters;
    }

    /** @dev Retrieves assigned App and other cluster info by clusterID
     * @param clusterID unique id of cluster
     * returns tuple representation of a Cluster
     */
    function getCluster(bytes32 clusterID)
        external
        view
    returns (bytes32, bytes32, uint8, address, bytes32[], bytes32, uint, bytes32[], uint16[])
    {
        Cluster memory cluster = clusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        return (
            cluster.app.storageHash,
            cluster.app.storageReceipt,
            cluster.app.clusterSize,
            cluster.app.owner,
            cluster.app.pinToNodes,
            cluster.app.appID,

            cluster.genesisTime,
            cluster.nodeIDs,
            cluster.ports
        );
    }

    /** @dev Retrieves addresses and ports of cluster's workers
     * @param clusterID unique id of cluster
     */
    function getClusterWorkers(bytes32 clusterID)
        external
        view
    returns (bytes24[], uint16[])
    {
        Cluster memory cluster = clusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        bytes24[] memory addresses = new bytes24[](cluster.nodeIDs.length);
        for(uint8 i = 0; i < cluster.nodeIDs.length; i++) {
            addresses[i] = nodes[cluster.nodeIDs[i]].nodeAddress;
        }

        return (
            addresses,
            cluster.ports
        );
    }


    /** @dev Gets apps which not yet deployed anywhere
     * return (apps Swarm hashes, receipts, clusters sizes, developers addresses, number of pinned nodes, all pinned nodes)
     */
    function getEnqueuedApps()
        external
        view
    returns(bytes32[], bytes32[], uint8[], address[], uint256[], bytes32[])
    {
        bytes32[] memory storageHashes = new bytes32[](enqueuedApps.length);
        bytes32[] memory appIDs = new bytes32[](enqueuedApps.length);
        uint8[] memory clusterSizes = new uint8[](enqueuedApps.length);
        address[] memory owners = new address[](enqueuedApps.length);
        uint256[] memory numberOfPinnedNodes = new uint256[](enqueuedApps.length);

        // count all pinned nodes to create an array for them
        uint256 count = 0;
        for (uint i = 0; i < enqueuedApps.length; i++) {
            count = count + enqueuedApps[i].pinToNodes.length;
        }

        bytes32[] memory allPinToNodes = new bytes32[](count);

        // reuse variable to iterate over all pinned nodes
        count = 0;

        for (i = 0; i < enqueuedApps.length; i++) {
            App memory app = enqueuedApps[i];

            storageHashes[i] = app.storageHash;
            appIDs[i] = app.appID;
            clusterSizes[i] = app.clusterSize;
            owners[i] = app.owner;
            numberOfPinnedNodes[i] = app.pinToNodes.length;

            for (uint j = 0; j < app.pinToNodes.length; j++) {
                allPinToNodes[count] = app.pinToNodes[j];
                count++;
            }
        }

        return (storageHashes, appIDs, clusterSizes, owners, numberOfPinnedNodes, allPinToNodes);
    }

    /** @dev Gets nodes and clusters IDs
     * return (node IDs, cluster IDs)
     */
    function getNodesIds()
        external
        view
    returns(bytes32[])
    {
        return nodesIds;
    }

    function getClustersIds()
    external
    view
    returns(bytes32[])
    {
        return clustersIds;
    }
}
