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

const crypto = require("crypto");
const assert = require("chai").assert;

exports.newNodeEvent = 'NewNode';
exports.nodeDeletedEvent = 'NodeDeleted';
exports.appEnqueuedEvent = 'AppEnqueued';
exports.appDeployedEvent = 'AppDeployed';
exports.appDeletedEvent = 'AppDeleted';
exports.appDequeuedEvent = 'AppDequeued';

function string2Bytes32 (str) {
    // 64 is for 32 bytes, 2 chars each
    // + 2 is for '0x' prefix
    return web3.padRight(web3.toHex(str), 64 + 2)
}

exports.string2Bytes32 = string2Bytes32;

function bytes32ToString (hex) {
    return web3.toUtf8(hex);
}

exports.bytes32ToString = bytes32ToString;

function generateNodeIDs(count) {
    return Array(count).fill(0).map(_ => string2Bytes32(crypto.randomBytes(16).hexSlice()));
}

exports.generateNodeIDs = generateNodeIDs;

// Adds new node
// count - number of nodes to add
// nodeIP - node IP address
// ownerAddress - node owner Ethereum account
// portCount - number of open ports, starting from 1000. i.e. [1000, 1000 + portCount - 1]
// private -- true if node is private; false if public
async function addNodes (contract, count, nodeIP, ownerAddress, portCount = 2, private = false) {
    assert(portCount > 0, "node should have at least single open port");
    
    return Promise.all(generateNodeIDs(count).map(
        async (nodeID) => {
            let receipt = await contract.addNode(
                nodeID,
                nodeIP,
                1000,
                1000 + portCount - 1,
                private,
                { from: ownerAddress }
            );
            
            return {
                nodeID: nodeID,
                receipt: receipt
            }
        }
    ))
}

exports.addNodesFull = addNodes

exports.addPinnedNodes = async function(contract, count, nodeIP, ownerAddress, portCount = 2, nodeIDs = []) {
    return addNodes(contract, count, nodeIP, ownerAddress, portCount, private = true, nodeIDs);
}

exports.addNodes = async function(contract, count, nodeIP, ownerAddress, portCount = 2) {
    return addNodes(contract, count, nodeIP, ownerAddress, portCount, private = false).then (result =>
        result.map(r => r.receipt)
    )
}

async function addApp (contract, count, owner, pinToNodes = []) {
    let storageHash = string2Bytes32(crypto.randomBytes(16).hexSlice());
    let storageReceipt = string2Bytes32(crypto.randomBytes(16).hexSlice());
    let receipt = await contract.addApp(storageHash, storageReceipt, count, pinToNodes, {from: owner});
    return {
        storageHash: storageHash,
        storageReceipt: storageReceipt,
        clusterSize: count,
        receipt: receipt
    }
}

exports.addApp = addApp;
