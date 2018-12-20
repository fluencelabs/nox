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

/*
 * This contract allows to:
 *  - register a node in Fluence network by submitting IP address and port range
 *  - deploy a code to Fluence network by submitting Swarm hash of the code and desired cluster size
 *
 * This contract also stores information about registered nodes, codes and their respective states.
 * Work horse of this contract is the `matchWork()` function that's called on new node and/or code registration.
 * When a code is matched with available nodes of desired quantity, `ClusterFormed` event is emitted and
 * is expected to trigger real-time cluster creation when received by matched Fluence nodes
 *
 */
contract Deployer is Whitelist {
    // Represents a Fluence Node which already is running or ready to run Solvers within the port range
    // Node's Solvers share the same Tendermint ID (consensus key) and nodeAddress
    struct Node {
        bytes32 id;
        bytes24 nodeAddress;
        uint16 startPort;
        uint16 endPort;
        uint16 currentPort;
        address owner;
        bool isPrivate;
    }

    // Represents deployed or enqueued (waiting to be deployed) code
    // code is stored in Swarm at storageHash, is deployed by developer
    // and requires to be hosted on cluster of clusterSize nodes
    struct Code {
        // code address in Swarm; also SwarmHash of the code
        // TODO: rename to codeAddress
        bytes32 storageHash;

        // Swarm receipt insuring code availability
        bytes32 storageReceipt;

        // number of real-time nodes required to host this code
        uint8 clusterSize;

        // ethereum address of the developer submitted that code
        address developer;

        // true if this code should be deployed only to nodes where node.owner == code.developer
        bytes32[] pinnedNodes;
    }

    struct BusyCluster {
        bytes32 clusterID;
        Code code;
        uint genesisTime;
        bytes32[] nodeIDs;
        bytes24[] nodeAddresses;
        uint16[] ports;
        address[] owners;
    }

    // Emitted when there is enough ready Nodes for some Code
    // Nodes' solvers should form a cluster in reaction to this event
    event ClusterFormed(
        bytes32 clusterID,
        bytes32 storageHash,
        uint genesisTime,
        bytes32[] solverIDs,
        bytes24[] solverAddrs,
        uint16[] solverPorts);

    // Emitted when Code is enqueued, telling that there is not enough Solvers yet
    event CodeEnqueued(bytes32 storageHash);

    // Emitted on every new Node
    event NewNode(bytes32 id);

    // Nodes ready to join new clusters
    bytes32[] internal readyNodes;
    uint32 readyNodesCount;

    // All nodes
    mapping(bytes32 => Node) internal nodes;
    bytes32[] internal nodesIndices;

    // Cluster with assigned Code
    mapping(bytes32 => BusyCluster) internal busyClusters;

    // Number of existing clusters, used for clusterID generation
    // starting with 1, so we could check existence of cluster in the mapping, e.g:
    // if (busyCluster[someId].clusterID > 0)
    uint256 clusterCount = 1;

    // Codes waiting for nodes
    Code[] internal enqueuedCodes;

    mapping(address => uint32) privateNodesCounter;

    /** @dev Adds node with specified port range to the work-waiting queue
      * @param nodeID some kind of unique ID
      * @param nodeAddress currently Tendermint p2p key + IP address, subject to change
      * @param startPort starting port for node's port range
      * @param endPort ending port for node's port range
      * emits NewNode event about new node
      * emits ClusterFormed event when there is enough nodes for some Code
      */
    function addNode(bytes32 nodeID, bytes24 nodeAddress, uint16 startPort, uint16 endPort, bool isPrivate)
        external
    {
        require(whitelist(msg.sender), "The sender is not in whitelist");
        require(nodes[nodeID].id == 0, "This node is already registered");

        // port range is inclusive
        // if startPort == endPort, then node can host just a single code
        require(startPort <= endPort, "Port range is empty or incorrect");

        nodes[nodeID] = Node(nodeID, nodeAddress, startPort, endPort, startPort, msg.sender, isPrivate);
        readyNodes.push(nodeID);
        nodesIndices.push(nodeID);

        if (isPrivate) {
            privateNodesCounter[msg.sender]++;
        } else {
            readyNodesCount++;
        }

        // match code to clusters until no matches left
        while (matchWork(isPrivate)) {}

        emit NewNode(nodeID);
    }

    /** @dev Adds new Code to be deployed on Solvers when there are enough of them
      * @param storageHash Swarm storage hash; allows code distributed and downloaded through it
      * @param storageReceipt Swarm receipt, serves as a proof that code is stored
      * @param clusterSize specifies number of Solvers that must serve Code
      * emits ClusterFormed event when there is enough nodes for the Code and emits CodeEnqueued otherwise, subject to change
      */
    function addCode(bytes32 storageHash, bytes32 storageReceipt, uint8 clusterSize, bytes32[] pinnedNodes)
        external
    {
        require(whitelist(msg.sender), "The sender is not in whitelist");
        require(pinnedNodes.length == 0 || clusterSize == pinnedNodes.length,
            "number of pinned nodes should be the same as desired clusterSize");

        enqueuedCodes.push(Code(storageHash, storageReceipt, clusterSize, msg.sender, pinnedNodes));

        if (!matchWork(pinnedNodes.length > 0)) {
            emit CodeEnqueued(storageHash);
        }
    }

    /** @dev Checks if there is enough free Nodes for not yet deployed Codes
     * @param pinned true if match private code to private nodes where node.owner == code.developer
     * emits ClusterFormed event if so
     */
    function matchWork(bool pinned)
        internal
        returns (bool)
    {
        int idx;
        if (pinned) {
            idx = findPinnedCode();
        } else {
            idx = findCode();
        }

        Code memory code;
        if (idx > 0) {
            code = enqueuedCodes[uint(idx)];
            removeCode(uint(idx));
        } else {
            return false;
        }

        // arrays containing nodes' data to be sent in a `ClusterFormed` event
        bytes32[] memory nodeIDs = new bytes32[](code.clusterSize);
        bytes24[] memory solverAddrs = new bytes24[](code.clusterSize);
        uint16[] memory solverPorts = new uint16[](code.clusterSize);
        address[] memory solverOwners = new address[](code.clusterSize);

        // i holds a position in readyNodes array
        uint i = 0;

        // j holds the number of currently collected nodes and a position in event data arrays
        for (uint8 j = 0; j < code.clusterSize; j++) {
            bytes32 nodeID = readyNodes[i];
            Node memory node = nodes[nodeID];

            // skip node if it doesn't fit the code
            if (!nodeFitsCode(node, code)) {
                i++;
                continue;
            }

            // copy node's data to arrays so it can be sent in event
            nodeIDs[j] = nodeID;
            solverAddrs[j] = node.nodeAddress;
            solverPorts[j] = node.currentPort;
            solverOwners[j] = node.owner;

            bool deleted = useNodePort(nodeID, i);
            // if deleted, useNodePort put last node in the array to i-th position
            // so if deleted, readyNodes[i] already contains 'new' node, so no need to increment i
            if (!deleted) i++;
        }

        // clusterID generation could be arbitrary, it doesn't depend on actual cluster count
        bytes32 clusterID = bytes32(clusterCount++);
        uint genesisTime = now;

        // saving selected nodes as a cluster with assigned code
        busyClusters[clusterID] = BusyCluster(clusterID, code, genesisTime, nodeIDs, solverAddrs, solverPorts, solverOwners);

        // notify Fluence node it's time to run real-time nodes and
        // create a Tendermint cluster hosting selected code
        emit ClusterFormed(clusterID, code.storageHash, genesisTime, nodeIDs, solverAddrs, solverPorts);
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
        enqueuedCodes.length--;
    }

    /** @dev Removes an element on specified position from 'readyNodes'
     * @param index position in 'readyNodes' to remove
     */
    function removeReadyNode(uint index)
        internal
    {
        if (index != readyNodes.length - 1) {
            // remove index-th node from readyNodes replacing it by the last node in the array
            readyNodes[index] = readyNodes[readyNodes.length - 1];
        }
        readyNodes.length--;
        readyNodesCount--;
    }

    function nodeFitsCode(Node node, Code code)
    internal
    pure
    returns (bool) {
        bool pinned = code.pinnedNodes.length > 0;
        if (pinned) {
            return node.isPrivate && node.owner == code.developer;
        }

        return !node.isPrivate;
    }

    /** @dev returns index of the first matched code
     * code should be from current msg.sender, i.e. code.developer == msg.sender
     * and there should be enough private nodes to host it
     */
    function findPinnedCode()
    internal
    view
    returns (int) {
        address developer = msg.sender;

        uint idx = 0;
        Code memory code;

        for (; idx < enqueuedCodes.length; idx++) {
            code = enqueuedCodes[idx];
            bool pinned = code.pinnedNodes.length > 0;

            // looking for pinned code from this developer
            // and enough private nodes to host the code
            if (
                pinned &&
                code.developer == developer &&
                code.clusterSize >= privateNodesCounter[developer] &&
                enoughPinnedNodes(code)
            ) { break; }
        }

        // check if we hit the condition above
        // idx >= enqueuedCodes.length means that we skipped through enqueuedCodes array without hitting condition
        if (idx >= enqueuedCodes.length) {
            return -1;
        }

        return int(idx);
    }

    /// @dev returns true if all pinned nodes are registered and have at least one open port
    function enoughPinnedNodes(Code code)
    internal
    view
    returns (bool)
    {
        require(code.pinnedNodes.length > 0, "code should have some pinned nodes");

        bool found = true;

        for (uint i = 0; i < code.pinnedNodes.length; i++) {
            bytes32 nodeID = code.pinnedNodes[i];
            Node memory node = nodes[nodeID];

            // node exists and has open ports
            bool free = node.nodeAddress > 0 && node.currentPort <= node.endPort;
            found = found && free;
        }

        return found;
    }

    /** @dev returns index of the first matched code
     * code shouldn't be pinned and there should be enough nodes to host it
     */
    function findCode()
    internal
    view
    returns (int) {
        uint idx = 0;
        Code memory code;

        // TODO: better control enqueuedCodes.length so we don't exceed gasLimit
        // looking for a code that can be deployed given current number of readyNodes
        for (; idx < enqueuedCodes.length; idx++) {
            code = enqueuedCodes[idx];
            bool pinned = code.pinnedNodes.length > 0;
            // ignoring pinned codes
            if (!pinned && readyNodesCount >= code.clusterSize) {
                // suitable code found, stop on current idx
                break;
            }
        }

        // check if we hit the condition `readyNodes.length >= code.clusterSize` above
        // idx >= enqueuedCodes.length means that we skipped through enqueuedCodes array without hitting condition
        if (idx >= enqueuedCodes.length) {
            return -1;
        }

        return int(idx);
    }

    /** @dev increments node's currentPort
     * and removes it from readyNodes if there are no more ports left
     * returns true if node was deleted from readyNodes
     */
    function useNodePort(bytes32 nodeID, uint readyNodeIdx)
    internal
    returns (bool)
    {
        // increment port, it will be used for the next code
        nodes[nodeID].currentPort++;

        Node memory node = nodes[nodeID];

        // check if node will be able to host a code next time; if no, remove it
        if (node.currentPort > node.endPort) {
            removeReadyNode(readyNodeIdx);

            if (node.isPrivate) {
                privateNodesCounter[node.owner]--;
            } else {
                readyNodesCount--;
            }

            return true;
        } else {
            return false;
        }
    }
}
