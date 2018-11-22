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

// TODO: comply to security suggestions from: https://github.com/OpenZeppelin/openzeppelin-solidity

// TODO: add pausing, circuit-breaking logic

// TODO: should this contract accept money?
// if no, reject payments.
// if yes, is it possible to introduce balance limit to avoid becoming high-profile contract? and thus target for attacks

// TODO: what are most critical invariants here?
// should we put a bug-bounty on them?

// TODO: what are gas usage goals/targets? is there any limit?
// TODO: calculate current gas usage

// TODO: should it be hash of the `storageHash`? so no one could download it
// in other words, is code private?

// Code:
// TODO: should storageHash be of type hash?
// TODO: should there be more statuses to just "deployed or not"?
// e.g 'deploying', 'deployed'
// maybe how many times it gets deployed, if that's the case

// TODO: there should be timeout on deployment status, and it should be confirmed periodically
// cuz it is possible for Solvers to ignore `CodeDeploying` while code is marked as deployed=true

// implementation is at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/contracts/access/Whitelist.sol
// example tests are at https://github.com/OpenZeppelin/openzeppelin-solidity/blob/master/test/ownership/Whitelist.test.js
import "openzeppelin-solidity/contracts/access/Whitelist.sol";

contract Deployer is Whitelist {
    // Represents a Fluence Node which already is running or ready to run Solvers within the port range
    // Node's Solvers share the same Tendermint ID (consensus key) and nodeAddress
    struct Node {
        bytes32 id;
        bytes24 nodeAddress;
        uint16 startPort;
        uint16 endPort;
        uint16 currentPort;
        uint solverClustersOffset;
    }

    // A single launched Solver described by its Node's Tendermint ID and assigned TCP port
    struct Solver {
        bytes32 id;
        uint16 port;
    }

    struct Code {
        bytes32 storageHash;
        bytes32 storageReceipt;
        uint8 clusterSize;
    }

    struct BusyCluster {
        bytes32 clusterID;
        Code code;
        uint genesisTime;
        uint busySolversOffset;
    }

    // Emitted when there is enough ready Nodes for some Code
    // Nodes' solvers should form a cluster in reaction to this event
    event ClusterFormed(bytes32 clusterID, bytes32 storageHash, uint genesisTime,
        bytes32[] solverIDs, bytes24[] solverAddrs, uint16[] solverPorts);

    // Emitted when Code is enqueued, telling that there is not enough Solvers yet
    event CodeEnqueued(bytes32 storageHash);

    // Emitted on every new Node
    event NewNode(bytes32 id);

    // Nodes ready to join new clusters
    bytes32[] private readyNodes;

    // All nodes
    mapping(bytes32 => Node) private nodes;

    // Array with actual cluster participants
    Solver[] private busySolvers;

    // Cluster with assigned Code
    mapping(bytes32 => BusyCluster) private busyClusters;

    // Array with formed solvers' clusters
    bytes32[] private solverClusters;

    // Number of existing clusters, used for clusterID generation
    // starting with 1, so we could check existince of cluster in the mapping, e.g:
    // if (busyCluster[someId].clusterID > 0)
    uint256 clusterCount = 1;

    // Codes waiting for nodes
    Code[] private enqueuedCodes;

    /** @dev Adds node with specified port range to the work-waiting queue
      * @param nodeID some kind of unique ID
      * @param nodeAddress currently Tendermint p2p key + IP address, subject to change
      * @param startPort starting port for node's port range
      * @param endPort ending port for node's port range
      * emits NewNode event about new node
      * emits ClusterFormed event when there is enough nodes for some Code
      */
    function addNode(bytes32 nodeID, bytes24 nodeAddress, uint16 startPort, uint16 endPort)
        external
        //onlyIfWhitelisted(msg.sender)
    {
        require(whitelist(msg.sender), "The sender is not in whitelist");
        require(nodes[nodeID].id == 0, "This node is already registered");
        require(startPort < endPort, "Port range is empty or incorrect");

        nodes[nodeID] = Node(nodeID, nodeAddress, startPort, endPort, startPort, solverClusters.length);
        readyNodes.push(nodeID);
        solverClusters.length += endPort - startPort;
        emit NewNode(nodeID);

        while (matchWork()) {
            // try match work while it matched
        }
    }

    /** @dev Adds new Code to be deployed on Solvers when there are enough of them
      * @param storageHash Swarm storage hash; allows code distributed and downloaded through it
      * @param storageReceipt Swarm receipt, serves as a proof that code is stored
      * @param clusterSize specifies number of Solvers that must serve Code
      * emits ClusterFormed event when there is enough nodes for the Code and emits CodeEnqueued otherwise, subject to change
      */
    function addCode(bytes32 storageHash, bytes32 storageReceipt, uint8 clusterSize)
        external
        //onlyIfWhitelisted(msg.sender)
    {
        require(whitelist(msg.sender), "The sender is not in whitelist");
        enqueuedCodes.push(Code(storageHash, storageReceipt, clusterSize));
        if (!matchWork()) {
            emit CodeEnqueued(storageHash);
        }
    }

    /** @dev Allows anyone with clusterID to retrieve assigned Code
     * @param clusterID unique id of cluster
     */
    function getCluster(bytes32 clusterID)
        external
        view
        returns (bytes32, bytes32, uint, bytes32[], bytes24[], uint16[])
    {
        BusyCluster memory cluster = busyClusters[clusterID];
        require(cluster.clusterID > 0, "there is no such cluster");

        bytes32[] memory solverIDs = new bytes32[](cluster.code.clusterSize);
        bytes24[] memory solverAddrs = new bytes24[](cluster.code.clusterSize);
        uint16[] memory solverPorts = new uint16[](cluster.code.clusterSize);

        for (uint i = 0; i < cluster.code.clusterSize; i++) {
            Solver memory solverInstance = busySolvers[cluster.busySolversOffset + i];
            solverIDs[i] = solverInstance.id;
            solverAddrs[i] = nodes[solverInstance.id].nodeAddress;
            solverPorts[i] = solverInstance.port;
        }
        return (cluster.code.storageHash, cluster.code.storageReceipt, cluster.genesisTime,
            solverIDs, solverAddrs, solverPorts);
    }

    /** @dev Allows to track currently running clusters for specified node's solvers
     * @param nodeID ID of node (Tendermint consensus key)
     */
    function getNodeClusters(bytes32 nodeID)
        external
        view
        returns (bytes32[])
    {
        Node memory node = nodes[nodeID];
        bytes32[] memory clusters = new bytes32[](node.currentPort - node.startPort);
        for (uint i = 0; i < clusters.length; i++) {
            clusters[i] = solverClusters[node.solverClustersOffset + i];
        }
        return clusters;
    }

    /** @dev Allows to track contract status
     * return (contract version const, number of ready nodes, enqueued codes' lengths)
     */
    function getStatus()
        external
        view
        returns (uint8, uint256, uint256[])
    {
        uint256[] memory cs = new uint256[](enqueuedCodes.length);
        for (uint j = 0; j < enqueuedCodes.length; ++j) {
            cs[j] = enqueuedCodes[j].clusterSize;
        }
        // fast way to check if contract was deployed incorrectly: in this case getStatus() returns (0, 0, [])
        uint8 version = 101;
        return (version, readyNodes.length, cs);
    }

    /** @dev Checks if there is enough free Solvers for undeployed Code
     * emits ClusterFormed event if so
     */
    function matchWork()
        internal
        returns (bool)
    {
        uint idx = 0;
        // TODO: better control enqueuedCodes.length so we don't exceed gasLimit
        // maybe separate deployed and undeployed code in two arrays
        for (; idx < enqueuedCodes.length; ++idx) {
            if (readyNodes.length >= enqueuedCodes[idx].clusterSize) {
                break;
            }
        }

        // check if we hit the condition `readyNodes.length >= enqueuedCodes[idx].clusterSize` above
        // idx >= enqueuedCodes.length means that we skipped through enqueuedCodes array without hitting condition
        if (idx >= enqueuedCodes.length) {
            return false;
        }

        Code memory code = enqueuedCodes[idx];
        removeCode(idx);

        bytes32 clusterID = bytes32(clusterCount++);
        uint time = now;
        busyClusters[clusterID] = BusyCluster(clusterID, code, time, busySolvers.length);

        bytes32[] memory solverIDs = new bytes32[](code.clusterSize);
        bytes24[] memory solverAddrs = new bytes24[](code.clusterSize);
        uint16[] memory solverPorts = new uint16[](code.clusterSize);

        uint nodeIndex = 0;
        for (uint j = 0; j < code.clusterSize; j++) {
            bytes32 nodeID = readyNodes[nodeIndex];
            Node memory node = nodes[nodeID];

            Solver memory solverInstance = Solver(nodeID, node.currentPort);
            busySolvers.push(solverInstance);
            solverClusters[node.solverClustersOffset + node.currentPort - node.startPort] = clusterID;

            solverIDs[j] = nodeID;
            solverAddrs[j] = node.nodeAddress;
            solverPorts[j] = solverInstance.port;

            if (nextPort(nodeID)) {
                ++nodeIndex;
            } else {
                removeNode(nodeIndex);
            }
        }
        emit ClusterFormed(clusterID, code.storageHash, time, solverIDs, solverAddrs, solverPorts);
        return true;
    }

    /** @dev Removes an element on specified position from 'enqueuedCodes'
     * @param index position in 'enqueuedCodes' to remove
     */
    function removeCode(uint index)
        internal
    {
        if (index != enqueuedCodes.length - 1) {
            // remove index-th code from enqueuedCodes replacing it by the last code in the array
            enqueuedCodes[index] = enqueuedCodes[enqueuedCodes.length - 1];
        }
        --enqueuedCodes.length;
    }

    /** @dev Removes an element on specified position from 'readyNodes'
     * @param index position in 'readyNodes' to remove
     */
    function removeNode(uint index)
        internal
    {
        if (index != readyNodes.length - 1) {
            // remove index-th node from readyNodes replacing it by the last node in the array
            readyNodes[index] = readyNodes[readyNodes.length - 1];
        }
        --readyNodes.length;
    }

    /** @dev Switches 'currentPort' for specified node
     * @param nodeID of target node
     * returns whether there are more available ports
     */
    function nextPort(bytes32 nodeID)
        internal
        returns (bool)
    {
        nodes[nodeID].currentPort++;
        return nodes[nodeID].currentPort != nodes[nodeID].endPort;
    }
}
