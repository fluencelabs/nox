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
 * This contract allows to inspect Fluence network state by providing different getter methods.
 *
 * All information is stored in the Deployer and this contract just transforms it into tool-readable form.
 * Main consumers of this contract are Fluence command line utilities and web interfaces. So while it can be used by
 * humans directly, it's not designed for that purpose.
 *
 */
contract Network is Deployer {
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


    /** @dev Allows to track currently running clusters for specified node's workers
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

    /** @dev Allows anyone with clusterID to retrieve assigned App
     * @param clusterID unique id of cluster
     * returns tuple representation of a Cluster
     */
    function getCluster(bytes32 clusterID)
        external
        view
    returns (bytes32, bytes32, uint8, address, bytes32[], uint, bytes32[], uint16[])
    {
        Cluster memory cluster = clusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        return (
            cluster.app.storageHash,
            cluster.app.storageReceipt,
            cluster.app.clusterSize,
            cluster.app.owner,
            cluster.app.pinToNodes,

            cluster.genesisTime,
            cluster.nodeIDs,
            cluster.ports
        );
    }

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


    /** @dev Gets codes which not yet deployed anywhere
     * return (codes' Swarm hashes, receipts, clusters' sizes, developers' addresses)
     */
    function getEnqueuedApps()
        external
        view
    returns(bytes32[], bytes32[], uint8[], address[], uint256[], bytes32[])
    {
        bytes32[] memory storageHashes = new bytes32[](enqueuedApps.length);
        bytes32[] memory storageReceipts = new bytes32[](enqueuedApps.length);
        uint8[] memory clusterSizes = new uint8[](enqueuedApps.length);
        address[] memory owners = new address[](enqueuedApps.length);
        uint256[] memory numberOfPinnedNodes = new uint256[](enqueuedApps.length);

        uint256 count = 0;
        for (uint i = 0; i < enqueuedApps.length; i++) {
            count = count + app.pinToNodes.length;
        }

        bytes32[] memory allPinToNodes = new bytes32[](count);

        count = 0;

        for (i = 0; i < enqueuedApps.length; i++) {
            App memory app = enqueuedApps[i];

            storageHashes[i] = app.storageHash;
            storageReceipts[i] = app.storageReceipt;
            clusterSizes[i] = app.clusterSize;
            owners[i] = app.owner;
            numberOfPinnedNodes[i] = app.pinToNodes.length;

            for (uint j = 0; j < app.pinToNodes.length; j++) {
                allPinToNodes[count] = app.pinToNodes[j];
                count = count + 1;
            }
        }

        return (storageHashes, storageReceipts, clusterSizes, owners, numberOfPinnedNodes, allPinToNodes);
    }

    /** @dev Gets nodes and clusters IDs
     * return (node IDs, cluster IDs)
     */
    function getIds()
        external
        view
    returns(bytes32[], bytes32[])
    {
        return (nodesIds, clustersIds);
    }

}
