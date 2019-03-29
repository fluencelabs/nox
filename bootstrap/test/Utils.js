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

exports.generateNodeIDs = generateNodeIDs;
exports.ip2Bytes24 = ip2Bytes24;
exports.bytes2Ip = bytes2Ip;

const string2Bytes32 = web3.utils.asciiToHex;
const bytes32ToString = web3.utils.toUtf8;

exports.string2Bytes32 = string2Bytes32;
exports.bytes32ToString = bytes32ToString;

const StorageIpfs = web3.utils.padRight(web3.utils.fromDecimal(1), 64);
const StorageSwarm = web3.utils.padRight(web3.utils.fromDecimal(0), 64);
exports.StorageIpfs = StorageIpfs;
exports.StorageSwarm = StorageSwarm;

function generateNodeIDs(count) {
    return Array(count).fill(0).map(() => string2Bytes32(crypto.randomBytes(16).hexSlice()));
}

function ip2Bytes24(ip) {
    let hex = web3.utils.bytesToHex(ip.split(".").map(s => parseInt(s)));
    return web3.utils.padLeft(hex, 48);
}

function bytes2Ip(hex) {
    let nozeros = hex.replace(/^0x0+/,"0x");
    let bytes = web3.utils.hexToBytes(nozeros);
    return bytes.map(b => b.toString(10)).join(".");
}

// Adds new node
// count - number of nodes to add
// nodeIP - node IP address
// ownerAddress - node owner Ethereum account
// portCount - number of open ports, starting from 1000. i.e. [1000, 1000 + portCount - 1]
// private -- true if node is private; false if public
async function addNodes(contract, count, nodeIP, ownerAddress, portCount = 2, isPrivate = false) {
    assert(portCount > 0, "node should have at least single open port");

    return Promise.all(generateNodeIDs(count).map(
        async (nodeID) => {
            let receipt = await contract.addNode(
                nodeID,
                ip2Bytes24(nodeIP),
                1000,
                portCount,
                isPrivate,
                {from: ownerAddress}
            );

            return {
                nodeID: nodeID,
                receipt: receipt,
                logs: receipt.logs
            }
        }
    ))
}

exports.addNodesFull = addNodes;

exports.addPinnedNodes = async function (contract, count, nodeIP, ownerAddress, portCount = 2, nodeIDs = []) {
    return addNodes(contract, count, nodeIP, ownerAddress, portCount, isPrivate = true, nodeIDs);
};

exports.addNodes = async function (contract, count, nodeIP, ownerAddress, portCount = 2) {
    return addNodes(contract, count, nodeIP, ownerAddress, portCount, isPrivate = false).then(result =>
        result.map(r => r.receipt)
    )
};

async function addApp(contract, count, owner, pinToNodes = []) {
    let storageHash = string2Bytes32(crypto.randomBytes(16).hexSlice());
    let storageReceipt = string2Bytes32(crypto.randomBytes(16).hexSlice());
    let receipt = await contract.addApp(storageHash, storageReceipt, StorageIpfs, count, pinToNodes, {from: owner});
    return {
        storageHash: storageHash,
        storageReceipt: storageReceipt,
        storageType: StorageIpfs,
        clusterSize: count,
        receipt: receipt,
        logs: receipt.logs
    }
}

exports.addApp = addApp;
