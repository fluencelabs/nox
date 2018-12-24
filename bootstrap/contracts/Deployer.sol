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

// TODO: what are the most critical invariants here?
// should we put a bug-bounty on them?

// TODO: what are the gas usage goals/targets? is there any limit?
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
        // Unique node's ID, user provided; actually it's Tendermint's ValidatorKey
        bytes32 id;

        // Publicly reachable & verifiable node address; has `node` prefix as `address` is a reserved word
        bytes24 nodeAddress;

        // Next port that could be used for running solver
        uint16 nextPort;
        // The last port of Node's dedicated range
        uint16 lastPort;

        // ethereum address of the miner which runs this node
        address owner;

        // True if this node can be used only by `owner`
        bool isPrivate;
    }

    // Represents deployed or enqueued (waiting to be deployed) code
    // code is stored in Swarm at storageHash, is deployed by developer
    // and requires to be hosted on cluster of clusterSize nodes
    struct App {
        // WASM code address in Swarm; also SwarmHash of the code
        bytes32 storageHash;

        // Swarm receipt insuring code availability
        bytes32 storageReceipt;

        // number of real-time nodes required to host this code
        uint8 clusterSize;

        // ethereum address of the developer submitted that code
        address owner;

        // list of owner's nodes where this code must be placed; length <= clusterSize
        bytes32[] pinToNodes;
    }

    struct Cluster {
        bytes32 clusterID;

        App app;

        uint genesisTime;

        bytes32[] nodeIDs;
        bytes24[] nodeAddresses;
        uint16[] ports;

        // TODO explain
        address[] owners;
    }

    // Emitted when there is enough ready Nodes for some Code
    // Nodes' solvers should form a cluster in reaction to this event
    event ClusterFormed(
        bytes32 clusterID,

        bytes32 storageHash,
        uint genesisTime,

        bytes32[] nodeIDs,
        bytes24[] nodeAddresses,
        uint16[] ports
    );

    // Emitted when Code is enqueued, telling that there is not enough Solvers yet
    event AppEnqueued(bytes32 storageHash);

    // Emitted on every new Node
    event NewNode(bytes32 id);

    // Nodes ready to join new clusters
    bytes32[] internal readyNodes;

    // All nodes
    mapping(bytes32 => Node) internal nodes;
    // Store nodes indices to traverse nodes mapping
    bytes32[] internal nodesIds;

    // Cluster with assigned Code
    mapping(bytes32 => Cluster) internal clusters;

    // Number of existing clusters, used for clusterID generation
    // starting with 1, so we could check existence of cluster in the mapping, e.g:
    // if (busyCluster[someId].clusterID > 0)
    uint256 clusterCount = 1;

    // Codes waiting for nodes
    App[] internal enqueuedApps;

    /** @dev Adds node with specified port range to the work-waiting queue
      * @param nodeID Tendermint's ValidatorKey
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

        nodes[nodeID] = Node(nodeID, nodeAddress, startPort, endPort, msg.sender, isPrivate);
        readyNodes.push(nodeID);
        nodesIds.push(nodeID);

        // match apps to the node until no matches left
        // TODO make a better data structure for enqueuedApps
        for(uint i = 0; i < enqueuedApps.length;) {
            App memory app = enqueuedApps[i];
            if(tryToDeployApp(app)) {
                // Once an app is deployed, we already have a new app on i-th position, so no need to increment i
                removeApp(i);

                // We should stop if there's no more ports in this node -- its addition has no more effect
                Node memory node = nodes[nodeID];
                if(node.nextPort > node.lastPort) break;
            } else i++;
        }

        emit NewNode(nodeID);
    }

    /** @dev Adds new App to be deployed on Nodes when there are enough of them
      * @param storageHash Swarm storage hash; allows code distributed and downloaded through it
      * @param storageReceipt Swarm receipt, serves as a proof that code is stored
      * @param clusterSize specifies number of Solvers that must serve the App
      * @param pinToNodes list of msg.sender's nodes where the App must reside
      * emits ClusterFormed event when there is enough nodes for the App and emits CodeEnqueued otherwise, subject to change
      */
    function addApp(bytes32 storageHash, bytes32 storageReceipt, uint8 clusterSize, bytes32[] pinToNodes)
        external
    {
        require(whitelist(msg.sender), "The sender is not in whitelist");
        require(clusterSize >= pinToNodes.length,
            "number of pinned nodes should be less or equal to the desired clusterSize");

        App memory app = App(storageHash, storageReceipt, clusterSize, msg.sender, pinToNodes);

        tryToDeployApp(app);
    }

    function tryToDeployApp(App memory app)
        internal
    returns(bool)
    {
        Node[] memory solvers = new Node[](app.clusterSize);
        uint8 solversLength = 0;

        bool skip = false;

        uint8 i = 0;
        uint j = 0;
        Node memory node;

        for(; i < app.pinToNodes.length; i++) {
            skip = false;
            // It should be better then making a custom data structure due to small expected size of pinToNodes
            for(; j < i && !skip; j++) {
                skip = app.pinToNodes[i] == app.pinToNodes[j];
            }
            if(skip) {
                continue;
            }
            node = nodes[app.pinToNodes[i]];
            if(node.owner == msg.sender) {
                solvers[solversLength] = node;
                solversLength++;
            }
        }

        for(j = 0; j < readyNodes.length && solvers.length < app.clusterSize; j++) {
            node = nodes[readyNodes[j]];

            // Skip all the private nodes; they should be used only with explicit pinning
            skip = node.isPrivate;
            // It should work better then a custom data structure due to high storage costs & small solvers size expectations
            for(i = 0; i < solvers.length && !skip; i++) {
                if(solvers[i].id == node.id) skip = true;
            }
            if(skip) {
                continue;
            }
            solvers[solversLength] = node;
            solversLength++;
        }

        if(solversLength == app.clusterSize) {
            formCluster(app, solvers);
            return true;
        } else {
            enqueuedApps.push(app);
            emit AppEnqueued(app.storageHash);
            return false;
        }
    }

    /**
     * @dev Forms a cluster, emits ClusterFormed event
     */
    function formCluster(App memory app, Node[] memory solvers)
        internal
    {
        require(app.clusterSize == solvers.length, "There should be enough nodes to form a cluster");

        // arrays containing nodes' data to be sent in a `ClusterFormed` event
        bytes32[] memory nodeIDs = new bytes32[](app.clusterSize);
        bytes24[] memory solverAddrs = new bytes24[](app.clusterSize);
        uint16[] memory solverPorts = new uint16[](app.clusterSize);
        address[] memory solverOwners = new address[](app.clusterSize);

        // j holds the number of currently collected nodes and a position in event data arrays
        for (uint8 j = 0; j < app.clusterSize; j++) {
            Node memory node = solvers[j];

            // copy node's data to arrays so it can be sent in event
            nodeIDs[j] = node.id;
            solverAddrs[j] = node.nodeAddress;
            solverPorts[j] = node.nextPort;
            solverOwners[j] = node.owner;

            useNodePort(node.id);
        }

        // clusterID generation could be arbitrary, it doesn't depend on actual cluster count
        bytes32 clusterID = bytes32(clusterCount++);
        uint genesisTime = now;

        // saving selected nodes as a cluster with assigned code
        clusters[clusterID] = Cluster(clusterID, app, genesisTime, nodeIDs, solverAddrs, solverPorts, solverOwners);

        // notify Fluence node it's time to run real-time nodes and
        // create a Tendermint cluster hosting selected code
        emit ClusterFormed(clusterID, app.storageHash, genesisTime, nodeIDs, solverAddrs, solverPorts);
    }

    /** @dev increments node's currentPort
     * and removes it from readyNodes if there are no more ports left
     * returns true if node was deleted from readyNodes
     */
    function useNodePort(bytes32 nodeID)
        internal
    returns (bool)
    {
        // increment port, it will be used for the next code
        nodes[nodeID].nextPort++;

        Node memory node = nodes[nodeID];

        // check if node will be able to host a code next time; if no, remove it
        if (node.nextPort > node.lastPort) {
            uint readyNodeIdx = 0;
            for(; readyNodeIdx < readyNodes.length; readyNodeIdx++) {
                if(readyNodes[readyNodeIdx] == node.id) {
                    removeReadyNode(readyNodeIdx);
                    break;
                }
            }

            return true;
        } else {
            return false;
        }
    }


    /** @dev Removes an element on specified position from 'readyNodes'
     *  @param index position in 'readyNodes' to remove
     */
    function removeReadyNode(uint index)
        internal
    {
        if (index != readyNodes.length - 1) {
            // remove index-th node from readyNodes replacing it by the last node in the array
            readyNodes[index] = readyNodes[readyNodes.length - 1];
        }
        // release the storage
        delete readyNodes[readyNodes.length - 1];

        readyNodes.length--;
    }


    /** @dev Removes an element on specified position from 'enqueuedApps'
     * @param index position in 'enqueuedApps' to remove
     */
    function removeApp(uint index)
        internal
    {
        if (index != enqueuedApps.length - 1) {
            // remove index-th code from enqueuedApps replacing it by the last code in the array
            enqueuedApps[index] = enqueuedApps[enqueuedApps.length - 1];
        }
        // release the storage
        delete enqueuedApps[enqueuedApps.length - 1];

        enqueuedApps.length--;
    }
}
