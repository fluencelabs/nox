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

var FluenceContract = artifacts.require("./Network.sol");
const truffleAssert = require('truffle-assertions');
const assert = require("chai").assert;
const crypto = require("crypto");
const should = require('chai').should();
const { expectThrow } = require('openzeppelin-solidity/test/helpers/expectThrow');

const newNodeEvent = 'NewNode';
const codeEnqueuedEvent = 'CodeEnqueued';
const clusterFormedEvent = 'ClusterFormed';

function string2Bytes32(str) {
    // 64 is for 32 bytes, 2 chars each
    // + 2 is for '0x' prefix
    return web3.padRight(web3.toHex(str), 64 + 2)
}

// Adds new node
// count - number of nodes to add
// nodeIP - node IP address
// ownerAddress - node owner Ethereum account
// portCount - number of open ports, starting from 1000. i.e. [1000, 1000 + portCount - 1]
async function addNodes(contract, count, nodeIP, ownerAddress, portCount = 2) {
    assert(portCount > 0, "node should have at least single open port")
    return Promise.all(Array(count).fill(0).map(
        (_, index) => {
            return contract.addNode(
                crypto.randomBytes(16).hexSlice(),
                nodeIP,
                1000,
                1000 + portCount - 1,
                { from: ownerAddress }
            )
        }
    ))
}

contract('Fluence', function ([_, owner, whitelisted, anyone]) {
    beforeEach(async function() {
      this.contract = await FluenceContract.new({ from: owner });
    });

    it("Should send event about new Node", async function() {
        let id = string2Bytes32("1");
        let receipt = await this.contract.addNode(id, "127.0.0.1", 1000, 1001, {from: whitelisted});
        truffleAssert.eventEmitted(receipt, newNodeEvent, (ev) => {
            assert.equal(ev.id, id);
            return true
        })
    });

    it("Should send event about enqueued Code", async function() {
        let storageHash = string2Bytes32("abc");
        let receipt = await this.contract.addCode(storageHash, "bca", 5, {from: whitelisted});

        truffleAssert.eventEmitted(receipt, codeEnqueuedEvent, (ev) => {
            assert.equal(ev.storageHash, storageHash);
            return true
        })
    });

    it("Should throw an error if asking about non-existent cluster", async function() {
        await expectThrow(
            this.contract.getCluster("abc")
        )
    });

    it("Should deploy code when there are enough nodes", async function() {
        let count = 5;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        let receipts = await addNodes(this.contract, count, "127.0.0.1", whitelisted);

        truffleAssert.eventEmitted(receipts.pop(), clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            clusterID = ev.clusterID;
            return true;
        });

        let cluster = await this.contract.getCluster(clusterID);
        assert.equal(cluster[0], storageHash);
        assert.equal(cluster[1], storageReceipt);

        let owners = cluster[6]; // eth addresses of node's owners
        assert.equal(owners.length, 5);
        owners.forEach(o => assert.equal(o, whitelisted)) // all nodes were registered from the same address
    });

    it("Should not form cluster from solvers of same node", async function() {
        await this.contract.addNode(crypto.randomBytes(16).hexSlice(), "127.0.0.1", 1000, 1002, {from: whitelisted});

        let count = 2;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        let receipt = await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        truffleAssert.eventEmitted(receipt, codeEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt, clusterFormedEvent)
    });

    it("Should reuse node until port range exhausted", async function() {
        let count = 1;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");

        let nodeID = crypto.randomBytes(16).hexSlice();

        await this.contract.addNode(nodeID, "127.0.0.1", 1000, 1001, {from: whitelisted});

        let receipt1 = await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        truffleAssert.eventNotEmitted(receipt1, codeEnqueuedEvent);
        truffleAssert.eventEmitted(receipt1, clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            assert.equal(ev.solverPorts[0], 1000);
            clusterID1 = ev.clusterID;
            return true;
        });

        let receipt2 = await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        truffleAssert.eventNotEmitted(receipt2, codeEnqueuedEvent);
        truffleAssert.eventEmitted(receipt2, clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            assert.equal(ev.solverPorts[0], 1001);
            clusterID2 = ev.clusterID;
            return true;
        });

        let receipt3 = await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        truffleAssert.eventEmitted(receipt3, codeEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt3, clusterFormedEvent);

        let nodeClusters = await this.contract.getNodeClusters(nodeID);

        assert.equal(nodeClusters[0], clusterID1);
        assert.equal(nodeClusters[1], clusterID2)
    });

    it("Should get correct list of clusters and enqueued codes", async function() {
        let [count1, count2, count3, count4] = [1, 2, 3, 4];

        let [storageHash1, storageHash2, storageHash3, storageHash4] =
            ["abc","abcd","abcde","abcdef"].map(s => string2Bytes32(s));

        let [storageReceipt1, storageReceipt2, storageReceipt3, storageReceipt4] =
            ["xyz","xyzd","xyzde","xyzdef"].map(s => string2Bytes32(s));

        await this.contract.addCode(storageHash1, storageReceipt1, count1, {from: whitelisted});
        await this.contract.addCode(storageHash2, storageReceipt2, count2, {from: whitelisted});
        await this.contract.addCode(storageHash3, storageReceipt3, count3, {from: whitelisted});
        await this.contract.addCode(storageHash4, storageReceipt4, count4, {from: whitelisted});

        await addNodes(this.contract, 3, "127.0.0.1", whitelisted, portCount = 2);

        let enqueuedCodes = await this.contract.getEnqueuedCodes();

        assert.equal(enqueuedCodes.length, 4); // storageHashes, storageReceipts, clusterSizes, developerAddresses

        let storageHashes = enqueuedCodes[0];
        let storageReceipts = enqueuedCodes[1];
        let clusterSizes = enqueuedCodes[2];
        let developerAddresses = enqueuedCodes[3];

        assert.equal(storageHashes.length, 2);
        assert.equal(storageHashes[0], storageHash4);
        assert.equal(storageHashes[1], storageHash3);

        assert.equal(storageReceipts.length, 2);
        assert.equal(storageReceipts[0], storageReceipt4);
        assert.equal(storageReceipts[1], storageReceipt3);

        assert.equal(clusterSizes.length, 2);
        assert.equal(clusterSizes[0], count4);
        assert.equal(clusterSizes[1], count3);

        assert.equal(developerAddresses.length, 2);
        assert.equal(developerAddresses[0], whitelisted);
        assert.equal(developerAddresses[1], whitelisted);

        let clustersInfos = await this.contract.getClustersInfo();
        let clustersNodes = await this.contract.getClustersNodes();

        assert.equal(clustersInfos[0].length, 2);
        assert.equal(clustersNodes[0].length, 3);

        let nodes = await this.contract.getReadyNodes();
        assert.equal(nodes[0].length, 3)
    });

    it("Should deploy same code twice", async function() {
        let count = 5;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        await this.contract.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        let firstCluster = (await addNodes(this.contract, count, "127.0.0.1", whitelisted, portCount = 1)).pop();
        let secondCluster = (await addNodes(this.contract, count, "127.0.0.1", whitelisted, portCount = 1)).pop();

        truffleAssert.eventEmitted(firstCluster, clusterFormedEvent, _ => true);
        truffleAssert.eventEmitted(secondCluster, clusterFormedEvent, _ => true)
    });
});
