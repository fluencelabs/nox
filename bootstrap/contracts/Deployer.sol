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
// cuz it is possible for Workers to ignore `CodeDeploying` while code is marked as deployed=true

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
contract Deployer {
    // Represents a Fluence Node which already is running or ready to run Workers within the port range
    // Node's Workers share the same Tendermint ID (consensus key) and nodeAddress
    struct Node {
        // Unique node's ID, user provided; actually it's Tendermint's ValidatorKey
        bytes32 id;

        // Publicly reachable & verifiable node address; has `node` prefix as `address` is a reserved word
        bytes24 nodeAddress;

        // Next port that could be used for running worker
        uint16 nextPort;
        // The last port of Node's dedicated range
        uint16 lastPort;

        // ethereum address of the miner which runs this node
        address owner;

        // True if this node can be used only by `owner`
        bool isPrivate;

        // Apps hosted by this node
        uint64[] appIDs;
    }

    // Represents deployed or enqueued (waiting to be deployed) code
    // code is stored in Swarm at storageHash, is deployed by developer
    // and requires to be hosted on cluster of clusterSize nodes
    struct App {
        uint64 appID;

        // WASM code address in Swarm; also SwarmHash of the code
        bytes32 storageHash;

        // Swarm receipt insuring code availability
        bytes32 storageReceipt;

        // number of real-time nodes required to host this code
        uint8 clusterSize;

        // ethereum address of the developer submitted that code
        address owner;

        // list of owner's nodes where this code must be deployed; length <= clusterSize
        // can contain both private & non-private nodes
        bytes32[] pinToNodes;

        Cluster cluster;
    }

    struct Cluster {
        // Cluster created at
        uint genesisTime;

        // IDs of participating nodes
        bytes32[] nodeIDs;

        // Worker's ports for each node
        uint16[] ports;
    }

    // Emitted when there is enough Workers for some App
    // Nodes' workers should form a cluster in reaction to this event
    event AppDeployed(
        uint64 appID,

        bytes32 storageHash,
        uint genesisTime,

        bytes32[] nodeIDs,
        bytes24[] nodeAddresses,
        uint16[] ports
    );

    // Emitted when App is enqueued, telling that there is not enough Workers yet
    event AppEnqueued(
        uint64 appID,
        bytes32 storageHash,
        bytes32 storageReceipt,
        uint8 clusterSize,
        address owner,
        bytes32[] pinToNodes
    );

    // Emitted on every new Node
    event NewNode(bytes32 id);

    // Emitted when node was deleted from nodes mapping and from readyNodes if it was there
    event NodeDeleted(bytes32 id);

    // Emitted when app is removed from enqueuedApps by owner
    event AppDequeued(uint64 appID);

    // Emitted when running app was removed by app owner
    event AppDeleted(uint64 appID);

    // Address of the contract owner (the one who deployed this contract)
    address private _contractOwner;

    // Nodes ready to join new clusters
    bytes32[] public readyNodes;

    // All nodes
    mapping(bytes32 => Node) internal nodes;
    // Store nodes indices to traverse nodes mapping
    bytes32[] public nodesIds;

    // mapping of appID to App
    mapping(uint64 => App) internal apps;
    // Store app ids to traverse clusters mapping
    uint64[] public appIDs;

    // Apps waiting for nodes
    uint64[] public enqueuedApps;

    // Number of all ever existed apps, used for appID generation
    uint64 internal appsCount = 1;

    constructor () internal {
        // Save contract's owner
        _contractOwner = msg.sender;
    }

    /**
     * @return true if `msg.sender` is the owner of the contract.
     */
    function isContractOwner() public view returns (bool) {
        return msg.sender == _contractOwner;
    }

    /** @dev Adds node with specified port range to the work-waiting queue
      * @param nodeID Tendermint's ValidatorKey
      * @param nodeAddress currently Tendermint P2P node ID + IP address, subject to change
      * @param startPort starting port for node's port range
      * @param endPort ending port for node's port range
      * emits NewNode event about new node
      * emits ClusterFormed event when there is enough nodes for some Code
      */
    function addNode(bytes32 nodeID, bytes24 nodeAddress, uint16 startPort, uint16 endPort, bool isPrivate)
        external
    {
        require(nodes[nodeID].id == 0, "This node is already registered");

        // port range is inclusive
        // if startPort == endPort, then node can host just a single code
        require(startPort <= endPort, "Port range is empty or incorrect");

        // Save the node
        nodes[nodeID] = Node(nodeID, nodeAddress, startPort, endPort, msg.sender, isPrivate, new uint64[](0));
        nodesIds.push(nodeID);

        // No need to add private nodes to readyNodes, as they could only used with by-id pinning
        if(!isPrivate) readyNodes.push(nodeID);

        emit NewNode(nodeID);

        // match apps to the node until no matches left, or until this node ports range is exhausted
        for(uint i = 0; i < enqueuedApps.length;) {
            uint64 appID = enqueuedApps[i];
            App storage app = apps[appID];
            if(tryDeployApp(app)) {
                // Once an app is deployed, we already have a new app on i-th position, so no need to increment i
                removeEnqueuedApp(i);

                // We should stop if there's no more ports in this node -- its addition has no more effect
                Node storage node = nodes[nodeID];
                if(node.nextPort > node.lastPort) break;
            } else i++;
        }
    }

    /** @dev Adds new App to be deployed on Nodes when there are enough of them
      * @param storageHash Swarm storage hash; allows code distributed and downloaded through it
      * @param storageReceipt Swarm receipt, serves as a proof that code is stored
      * @param clusterSize specifies number of Workers that must serve the App
      * @param pinToNodes list of msg.sender's nodes where the App must reside
      * emits ClusterFormed event when there is enough nodes for the App and
      * emits AppEnqueued otherwise, subject to change
      */
    function addApp(bytes32 storageHash, bytes32 storageReceipt, uint8 clusterSize, bytes32[] pinToNodes)
        external
    {
        require(clusterSize > 0, "Cluster size must be a positive number");

        require(clusterSize >= pinToNodes.length,
            "number of pinTo nodes should be less or equal to the desired clusterSize");

        // Check that pinToNodes are distinct nodes owned by msg.sender
        for(uint8 i = 0; i < pinToNodes.length; i++) {
            bytes32 nodeID_i = pinToNodes[i];
            Node storage node = nodes[nodeID_i];
            require(node.owner != 0, "Can pin only to registered nodes");
            require(node.owner == msg.sender, "Can pin only to nodes you own");

            for(uint8 j = 0; j <= i; j++) {
                if(i != j) {
                    require(nodeID_i != pinToNodes[j], "Node ids to pin to must be unique, otherwise the deployment result could be unpredictable and unexpected");
                }
            }
        }

        App memory app = App(
            appsCount++,
            storageHash,
            storageReceipt,
            clusterSize,
            msg.sender,
            pinToNodes,
            Cluster(0, new bytes32[](0), new uint16[](0)) // TODO: this is awful
        );
        apps[app.appID] = app;
        appIDs.push(app.appID);

        if(!tryDeployApp(apps[app.appID])) {
            // App hasn't been deployed -- enqueue it to have it deployed later
            enqueuedApps.push(app.appID);
            emit AppEnqueued(app.appID, app.storageHash, app.storageReceipt, app.clusterSize, app.owner, app.pinToNodes);
        }
    }

    /** @dev Deletes app with appID from enqueued apps
      * You must be app's owner to delete it. Currently, nodes' ports aren't freed.
      * @param appID app to be deleted
      * emits AppDequeued event on successful deletion
      * reverts if you're not app owner
      * reverts if app not found
      */
    function dequeueApp(uint64 appID)
        external
    {
        uint i = indexOf(appID, enqueuedApps);

        require(i < enqueuedApps.length, "error deleting app: app not found");

        require(apps[appID].owner == msg.sender || isContractOwner(), "error deleting app: you must own the app to delete it");

        removeEnqueuedApp(i);

        emit AppDequeued(appID);
    }

    /** @dev Deletes cluster that hosts app appID
      * You must be app's owner to delete it. Currently, nodes' ports aren't freed.
      * @param appID app to be deleted
      * emits AppRemoved event on successful deletion
      * reverts if you're not app owner
      * reverts if app or cluster aren't not found
      * TODO: free nodes' ports after app deletion
      */
    function deleteApp(uint64 appID)
        external
    {
        App storage app = apps[appID];
        require(app.appID != 0, "error deleting app: cluster not found");
        require(app.appID == appID, "error deleting app: cluster hosts another app");
        require(app.owner == msg.sender || isContractOwner(), "error deleting app: you must own app to delete it");
        require(app.cluster.genesisTime != 0, "error deleting app: app must be deployed, use dequeueApp");

        // Remove appID from node.appIDs for all nodes that host that app
        removeAppFromNodes(appID, app.cluster.nodeIDs);

        // Remove app from clustersIds array and clusters mapping, and emit AppDeleted event of successful removal
        removeApp(appID);
    }

    /** @dev Removes appID from node.appIDs for nodes in nodeIDsArray.
      * Reverts if any node in nodeIDsArray doesn't contain appID in node.appIDs
      * @param appID appID to remove
      * @param nodeIDsArray list of nodes to remove appID from. Every node.appIDs in that list must contain that appID.
      */
    function removeAppFromNodes(uint64 appID, bytes32[] storage nodeIDsArray)
        internal
    {
        for (uint i = 0; i < nodeIDsArray.length; i++) {
            uint64[] storage appIDsArray = nodes[nodeIDsArray[i]].appIDs;
            uint idx = indexOf(appID, appIDsArray);
            require(idx < nodeIDsArray.length, "error deleting app: app not found in node.appIDs");
            removeArrayElement(idx, appIDsArray);
        }
    }

    /** @dev Deletes node from nodes mapping, nodeIds and readyNodes arrays
    * @param nodeID ID of the node to be deleted
    */
    function deleteNode(bytes32 nodeID)
        external
    {
        Node storage node = nodes[nodeID];
        require(node.id != 0, "error deleting node: node not found");
        require(node.owner == msg.sender || isContractOwner(), "error deleting node: you must own node to delete it");

        // Remove nodeID from apps that hosted by this node. Also remove app if there's no more nodes left to host them.
        removeNodeFromApps(nodeID, node.appIDs);

        // Find the node in readyNodes
        uint i = indexOf(nodeID, readyNodes);

        // If node was in readyNodes, remove it
        if (i < readyNodes.length) {
            removeReadyNode(i);
        }

        // Find the node in nodesIds
        i = indexOf(nodeID, nodesIds);
        // This should never happen if there's no bugs. But it's better to revert if there is some inconsistency.
        require(i < nodesIds.length, "error deleting node: node not found in nodesIds array");
        // Remove node from nodesIds array
        removeFromNodeIds(i);

        // Remove node from nodes mapping
        delete nodes[nodeID];

        emit NodeDeleted(nodeID);
    }

    /** @dev Remove nodeID from app.cluster.nodeIDs for apps hosted by this node.
      * Also remove app if there's no more nodes left to host it, and emit AppDeleted event.
      */
    function removeNodeFromApps(bytes32 nodeID, uint64[] storage appIDsArray)
        internal
    {
        for (uint i = 0; i < appIDsArray.length; i++) {
            uint64 appID = appIDsArray[i];
            bytes32[] storage appNodeIDs = apps[appID].cluster.nodeIDs;
            uint appNodeIDsLength = appNodeIDs.length;

            uint j = indexOf(nodeID, appNodeIDs);

            // This should never happen if there's no bugs. But it's better to revert if there is some inconsistency.
            require(j < appNodeIDsLength, "error deleting node: nodeID wasn't found in nodeIDs");

            // Check if that node is the last one hosting that app
            if (appNodeIDsLength == 1) {
                // Remove app from clustersIds array and clusters mapping, and emit AppDeleted event of successful removal
                removeApp(appID);
            } else {
                removeArrayElement(j, appNodeIDs);
            }
        }
    }

    /** @dev Tries to deploy an app, using ready nodes and their ports
      * @param app Application to deploy
      * emits AppDeployed when App is deployed
      */
    function tryDeployApp(App storage app)
        internal
    returns(bool)
    {
        // Number of collected workers
        uint8 workersCount = 0;

        // Array of workers that will be used to form a cluster
        bytes32[] memory workers = new bytes32[](app.clusterSize);

        // There must be enough readyNodes to try to deploy the app
        if(readyNodes.length >= app.clusterSize - app.pinToNodes.length) {
            // Index used to iterate through pinToNodes and then workers
            uint8 i = 0;

            // Current node to check
            Node storage node;

            // Find all the nodes where code should be pinned
            // Nodes in pinToNodes are already checked to belong to app owner
            // pinToNodes is already deduplicated in addApp
            for(; i < app.pinToNodes.length; i++) {
                node = nodes[app.pinToNodes[i]];

                // Return false if there's not enough capacity on pin-to node to deploy the app
                if(node.nextPort > node.lastPort) {
                    return false;
                }

                workers[workersCount] = node.id;
                workersCount++;
            }

            // Find ready nodes to pin to
            for(uint j = 0; j < readyNodes.length && workersCount < app.clusterSize; j++) {
                node = nodes[readyNodes[j]];

                // True if node is already in workers array. That could happen if
                // app.owner pinned app to non-private node
                // skip is used to avoid including such nodes twice
                bool skip = false;

                // That algorithm should work better than a custom data structure
                // due to high storage costs & small workers size expectations
                for(i = 0; i < workers.length && !skip; i++) {
                    if(workers[i] == node.id) skip = true;
                }

                if(skip) continue;

                workers[workersCount] = node.id;
                workersCount++;
            }
        }

        if(workersCount == app.clusterSize) {
            formCluster(app, workers);
            return true;
        }

        return false;
    }

    /**
     * @dev Forms a cluster, emits ClusterFormed event, marks workers' ports as used
     */
    function formCluster(App storage app, bytes32[] memory workers)
        internal
    {
        require(app.clusterSize == workers.length, "There should be enough nodes to form a cluster");

        // arrays containing nodes' data to be sent in a `ClusterFormed` event
        bytes32[] memory nodeIDs = new bytes32[](app.clusterSize);
        bytes24[] memory workerAddrs = new bytes24[](app.clusterSize);
        uint16[] memory workerPorts = new uint16[](app.clusterSize);

        // j holds the number of currently collected nodes and a position in event data arrays
        for (uint8 j = 0; j < app.clusterSize; j++) {
            Node storage node = nodes[workers[j]];

            // copy node's data to arrays so it can be sent in event
            nodeIDs[j] = node.id;
            workerAddrs[j] = node.nodeAddress;
            workerPorts[j] = node.nextPort;

            useNodePort(node.id);
            nodes[node.id].appIDs.push(app.appID);
        }

        uint genesisTime = now;

        // saving selected nodes as a cluster with assigned app
        app.cluster = Cluster(genesisTime, nodeIDs, workerPorts);

        // notify Fluence node it's time to run real-time workers and
        // create a Tendermint cluster hosting selected App (defined by storageHash)
        emit AppDeployed(app.appID, app.storageHash, genesisTime, nodeIDs, workerAddrs, workerPorts);
    }

    /** @dev increments node's currentPort
     * and removes it from readyNodes if there are no more ports left
     * returns true if node was deleted from readyNodes
     */
    function useNodePort(bytes32 nodeID)
        internal
    returns (bool)
    {

        Node storage node = nodes[nodeID];

        // increment port, it will be used for the next code
        node.nextPort++;

        // check if node will be able to host a code next time; if no, remove it
        if (node.nextPort > node.lastPort) {
            uint readyNodeIdx = indexOf(node.id, readyNodes);
            if (readyNodeIdx < readyNodes.length) {
                removeReadyNode(readyNodeIdx);
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
        removeArrayElement(index, readyNodes);
    }

    function removeFromNodeIds(uint index)
    internal
    {
        removeArrayElement(index, nodesIds);
    }


    /** @dev Removes an element on specified position from 'enqueuedApps'
     * @param index position in 'enqueuedApps' to remove
     */
    function removeEnqueuedApp(uint index)
        internal
    {
        removeArrayElement(index, enqueuedApps);
    }

    /** @dev Removes cluster from clustersIds array and clusters mapping and emits an event on successful App removal
     *  @param appID ID of the app to be removed
     *  returns true if cluster was deleted, false otherwise
     */
    function removeApp(uint64 appID)
        internal
    {
        // look for appID in appIDs array
        uint index = indexOf(appID, appIDs);

        // revert we didn't find such appID
        require(index < appIDs.length, "error deleting app: app not found in appIDs array");

        removeArrayElement(index, appIDs);

        // also remove cluster from mapping
        delete apps[appID];


        emit AppDeleted(appID);
    }

    function removeArrayElement(uint index, bytes32[] storage array)
        internal
    {
        uint lenSubOne = array.length - 1;
        if (index != lenSubOne) {
            // remove index-th element from array replacing it by the last element in the array
            array[index] = array[lenSubOne];
        }
        // release the storage
        delete array[lenSubOne];

        array.length--;
    }

    function removeArrayElement(uint index, uint64[] storage array)
    internal
    {
        uint lenSubOne = array.length - 1;
        if (index != lenSubOne) {
            // remove index-th element from array replacing it by the last element in the array
            array[index] = array[lenSubOne];
        }
        // release the storage
        delete array[lenSubOne];

        array.length--;
    }

    function indexOf(uint64 id, uint64[] storage array)
        internal view
    returns (uint)
    {
        uint i;
        // Find index of id in the array
        for(i = 0; i < array.length; i++) {
            if (array[i] == id) {
                break;
            }
        }

        return i;
    }

    function indexOf(bytes32 id, bytes32[] storage array)
        internal view
    returns (uint)
    {
        uint i;
        // Find index of id in the array
        for(i = 0; i < array.length; i++) {
            if (array[i] == id) {
                break;
            }
        }

        return i;
    }
}
