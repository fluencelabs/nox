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

    /** @dev Allows to track currently running clusters for specified node's solvers
     *  @param nodeID ID of node (Tendermint consensus key)
     *  returns IDs of clusters where the node is a member.
     */
    function getNodeClusters(bytes32 nodeID)
    external
    view
    returns (bytes32[])
    {
        Node memory node = nodes[nodeID];
        bytes32[] memory clusterIDs = new bytes32[](node.currentPort - node.startPort);
        uint count = 0;

        for (uint i = 1; i < clusterCount; i++) {
            BusyCluster memory cluster = busyClusters[bytes32(i)];
            for (uint j = 0; j < cluster.nodeAddresses.length; j++) {
                if (cluster.nodeAddresses[j] == node.nodeAddress) {
                    clusterIDs[count++] = cluster.clusterID;
                }
            }
        }
        return clusterIDs;
    }

    /** @dev Allows anyone with clusterID to retrieve assigned Code
     * @param clusterID unique id of cluster
     * returns tuple representation of a BusyCluster
     */
    function getCluster(bytes32 clusterID)
    external
    view
    returns (bytes32, bytes32, uint, bytes32[], bytes24[], uint16[], address[])
    {
        BusyCluster memory cluster = busyClusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        return (
            cluster.code.storageHash,
            cluster.code.storageReceipt,
            cluster.genesisTime,
            cluster.nodeIDs,
            cluster.nodeAddresses,
            cluster.ports,
            cluster.owners
        );
    }

    /** @dev Gets info about registered clusters
     * For full network state, use this method in conjunction with `getClustersNodes`
     * returns tuple representation of an array of cluster-related data from BusyClusters
     * (cluster IDs, genesis times, codes' Swarm hashes, receipts, clusters' sizes, developers of codes deployed on clusters)
     */
    function getClustersInfo()
    external
    view
    returns (bytes32[], uint[], bytes32[], bytes32[], uint8[], address[])
    {
        BusyCluster[] memory clusters = new BusyCluster[](clusterCount - 1);

        for (uint i = 1; i < clusterCount; i++) {
            clusters[i-1] = busyClusters[bytes32(i)];
        }

        bytes32[] memory clusterIDs = new bytes32[](clusters.length);
        uint[] memory genesisTimes = new uint[](clusters.length);
        bytes32[] memory storageHashes = new bytes32[](clusters.length);
        bytes32[] memory storageReceipts = new bytes32[](clusters.length);
        uint8[] memory clusterSizes = new uint8[](clusters.length);
        address[] memory developers = new address[](clusters.length);

        for (uint k = 0; k < clusters.length; k++) {
            BusyCluster memory cluster = clusters[k];
            clusterIDs[k] = cluster.clusterID;
            genesisTimes[k] = cluster.genesisTime;
            storageHashes[k] = cluster.code.storageHash;
            storageReceipts[k] = cluster.code.storageReceipt;
            clusterSizes[k] = cluster.code.clusterSize;
            developers[k] = cluster.code.developer;
        }

        return (clusterIDs, genesisTimes, storageHashes, storageReceipts, clusterSizes, developers);
    }

    /** @dev Gets nodes that already members in all registered clusters
     * For full network state, use this method in conjunction with `getClustersInfo`
     * returns tuple representation of an array of nodes-related data from BusyClusters
     * (ids, node addresses, ports, node owners ethereum addresses)
     */
    function getClustersNodes()
    external
    view
    returns (bytes32[], bytes24[], uint16[], address[])
    {
        BusyCluster[] memory clusters = new BusyCluster[](clusterCount - 1);
        uint solversCount = 0;
        for (uint i = 1; i < clusterCount; i++) {
            uint key = i-1;
            BusyCluster memory cl = busyClusters[bytes32(i)];
            clusters[key] = cl;
            solversCount = solversCount + cl.code.clusterSize;
        }

        bytes32[] memory ids = new bytes32[](solversCount);
        bytes24[] memory addresses = new bytes24[](solversCount);
        uint16[] memory ports = new uint16[](solversCount);
        address[] memory owners = new address[](solversCount);

        // solversCount is reused here to reduce stack depth
        solversCount = 0;

        for (uint k = 0; k < clusters.length; k++) {
            BusyCluster memory cluster = clusters[k];

            for (uint n = 0; n < cluster.nodeAddresses.length; n++) {
                ids[solversCount] = cluster.nodeIDs[n];
                addresses[solversCount] = cluster.nodeAddresses[n];
                ports[solversCount] = cluster.ports[n];
                owners[solversCount] = cluster.owners[n];
                solversCount++;
            }
        }

        return (ids, addresses, ports, owners);
    }

    /** @dev Gets codes which not yet deployed anywhere
     * return (codes' Swarm hashes, receipts, clusters' sizes, developers' addresses)
     */
    function getEnqueuedCodes()
    external
    view
    returns(bytes32[], bytes32[], uint8[], address[])
    {
        bytes32[] memory storageHashes = new bytes32[](enqueuedCodes.length);
        bytes32[] memory storageReceipts = new bytes32[](enqueuedCodes.length);
        uint8[] memory clusterSizes = new uint8[](enqueuedCodes.length);
        address[] memory developers = new address[](enqueuedCodes.length);

        for (uint i = 0; i < enqueuedCodes.length; i++) {
            Code memory code = enqueuedCodes[i];

            storageHashes[i] = code.storageHash;
            storageReceipts[i] = code.storageReceipt;
            clusterSizes[i] = code.clusterSize;
            developers[i] = code.developer;
        }

        return (storageHashes, storageReceipts, clusterSizes, developers);
    }

    /** @dev Gets nodes that have free ports to host code
     * returns tuple representation of a list of Node structs
     * (node IDs, nodes' addresses, starting ports, ending ports, current ports, nodes' owners)
     */
    function getReadyNodes()
    external
    view
    returns (bytes32[], bytes24[], uint16[], uint16[], uint16[], address[])
    {
        bytes32[] memory ids = new bytes32[](nodesIndices.length);
        bytes24[] memory nodeAddresses = new bytes24[](nodesIndices.length);
        uint16[] memory startPorts = new uint16[](nodesIndices.length);
        uint16[] memory endPorts = new uint16[](nodesIndices.length);
        uint16[] memory currentPorts = new uint16[](nodesIndices.length);
        address[] memory owners = new address[](nodesIndices.length);

        for (uint i = 0; i < nodesIndices.length; ++i) {
            Node memory node = nodes[nodesIndices[i]];
            ids[i] = node.id;
            nodeAddresses[i] = node.nodeAddress;
            startPorts[i] = node.startPort;
            endPorts[i] = node.endPort;
            currentPorts[i] = node.currentPort;
            owners[i] = node.owner;
        }

        return (ids, nodeAddresses, startPorts, endPorts, currentPorts, owners);
    }
}
