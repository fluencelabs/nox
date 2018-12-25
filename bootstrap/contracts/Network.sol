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
    returns (bytes32, bytes32, uint, bytes32[], bytes24[], uint16[], bytes32[])
    {
        Cluster memory cluster = clusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        bytes24[] memory addresses = new bytes24[](cluster.nodeIDs.length);
        for(uint8 i = 0; i < cluster.nodeIDs.length; i++) {
            addresses[i] = nodes[cluster.nodeIDs[i]].nodeAddress;
        }

        return (
            cluster.app.storageHash,
            cluster.app.storageReceipt,
            cluster.genesisTime,
            cluster.nodeIDs,
            addresses,
            cluster.ports,
            cluster.app.pinToNodes
        );
    }

    /** @dev Gets info about registered clusters
     * For full network state, use this method in conjunction with `getClustersNodes`
     * returns tuple representation of an array of cluster-related data from Clusters
     * (cluster IDs, genesis times, codes' Swarm hashes, receipts, clusters' sizes, developers of codes deployed on clusters)
     */
    function getClustersInfo()
    external
    view
    returns (bytes32[], uint[], bytes32[], bytes32[], uint8[], address[], bool[])
    {
        bytes32[] memory clusterIDs = new bytes32[](clusterCount - 1);
        uint[] memory genesisTimes = new uint[](clusterCount - 1);
        bytes32[] memory storageHashes = new bytes32[](clusterCount - 1);
        bytes32[] memory storageReceipts = new bytes32[](clusterCount - 1);
        uint8[] memory clusterSizes = new uint8[](clusterCount - 1);
        address[] memory developers = new address[](clusterCount - 1);
        bool[] memory isPrivate = new bool[](clusterCount - 1);

        for (uint k = 1; k < clusterCount; k++) {
            Cluster memory cluster = clusters[bytes32(k)];
            clusterIDs[k - 1] = cluster.clusterID;
            genesisTimes[k - 1] = cluster.genesisTime;
            storageHashes[k - 1] = cluster.app.storageHash;
            storageReceipts[k - 1] = cluster.app.storageReceipt;
            clusterSizes[k - 1] = cluster.app.clusterSize;
            developers[k - 1] = cluster.app.owner;
            isPrivate[k - 1] = cluster.app.pinToNodes.length > 0;
        }

        return (clusterIDs, genesisTimes, storageHashes, storageReceipts, clusterSizes, developers, isPrivate);
    }

    /** @dev Gets nodes that already members in all registered clusters
     * TODO do we really need this method? Its return type is very obscure
     * For full network state, use this method in conjunction with `getClustersInfo`
     * returns tuple representation of an array of nodes-related data from Clusters
     * (ids, node addresses, ports, node owners ethereum addresses)
     */
    function getClustersNodes()
    external
    view
    returns (bytes32[], uint16[])
    {
        Cluster[] memory collectedClusters = new Cluster[](clusterCount - 1);
        uint workersCount = 0;
        for (uint i = 1; i < clusterCount; i++) {
            uint key = i-1;
            Cluster memory cl = clusters[bytes32(i)];
            collectedClusters[key] = cl;
            workersCount = workersCount + cl.app.clusterSize;
        }

        bytes32[] memory ids = new bytes32[](workersCount);
        uint16[] memory ports = new uint16[](workersCount);

        // workersCount is reused here to reduce stack depth
        workersCount = 0;

        for (uint k = 0; k < collectedClusters.length; k++) {
            Cluster memory cluster = collectedClusters[k];

            for (uint n = 0; n < cluster.nodeIDs.length; n++) {
                ids[workersCount] = cluster.nodeIDs[n];
                ports[workersCount] = cluster.ports[n];
                workersCount++;
            }
        }

        return (ids, ports);
    }

    /** @dev Gets codes which not yet deployed anywhere
     * return (codes' Swarm hashes, receipts, clusters' sizes, developers' addresses)
     */
    function getEnqueuedApps()
    external
    view
    returns(bytes32[], bytes32[], uint8[], address[], bool[], bytes32[])
    {
        uint pinnedCount;

        for (uint i = 0; i < enqueuedApps.length; i++) {
            // What is it? What good it does?
            pinnedCount += app.pinToNodes.length;
        }

        bytes32[] memory storageHashes = new bytes32[](enqueuedApps.length);
        bytes32[] memory storageReceipts = new bytes32[](enqueuedApps.length);
        uint8[] memory clusterSizes = new uint8[](enqueuedApps.length);
        address[] memory developers = new address[](enqueuedApps.length);
        bool[] memory pinned = new bool[](enqueuedApps.length);
        bytes32[] memory pinnedNodes = new bytes32[](pinnedCount);

        pinnedCount = 0;

        for (i = 0; i < enqueuedApps.length; i++) {
            App memory app = enqueuedApps[i];

            storageHashes[i] = app.storageHash;
            storageReceipts[i] = app.storageReceipt;
            clusterSizes[i] = app.clusterSize;
            developers[i] = app.owner;
            pinned[i] = app.pinToNodes.length > 0;

            for (uint j = 0; j < app.pinToNodes.length; j++) {
                pinnedNodes[pinnedCount] = app.pinToNodes[j];
                pinnedCount++;
            }
        }

        return (storageHashes, storageReceipts, clusterSizes, developers, pinned, pinnedNodes);
    }

    /** @dev Gets nodes that have free ports to host code
     * returns tuple representation of a list of Node structs
     * (node IDs, nodes' addresses, ending ports, current ports, nodes' owners, isPrivate)
     */
    function getReadyNodes()
    external
    view
    returns (bytes32[], bytes24[], uint16[], uint16[], address[], bool[])
    {
        bytes32[] memory ids = new bytes32[](nodesIds.length);
        bytes24[] memory nodeAddresses = new bytes24[](nodesIds.length);
        uint16[] memory lastPorts = new uint16[](nodesIds.length);
        uint16[] memory nextPorts = new uint16[](nodesIds.length);
        address[] memory owners = new address[](nodesIds.length);
        bool[] memory isPrivate = new bool[](nodesIds.length);

        for (uint i = 0; i < nodesIds.length; ++i) {
            Node memory node = nodes[nodesIds[i]];
            ids[i] = node.id;
            nodeAddresses[i] = node.nodeAddress;
            lastPorts[i] = node.lastPort;
            nextPorts[i] = node.nextPort;
            owners[i] = node.owner;
            isPrivate[i] = node.isPrivate;
        }

        return (ids, nodeAddresses, lastPorts, nextPorts, owners, isPrivate);
    }
}
