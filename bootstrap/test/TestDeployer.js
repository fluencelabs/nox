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

var Deployer = artifacts.require("./Deployer.sol");
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

async function addNodes(instance, count, addr, adder) {
    return Promise.all(Array(count).fill(0).map(
        (_, index) => {
            return instance.addNode(crypto.randomBytes(16).hexSlice(), addr, 1000, 1001, { from: adder })
        }
    ))
}

contract('Deployer', function ([_, owner, whitelisted, anyone]) {
    beforeEach(async function() {
      this.deployer = await Deployer.new({ from: owner });
      await this.deployer.addAddressToWhitelist(whitelisted, { from: owner })
    });

    it("Should send event about new Node", async function() {
        let id = string2Bytes32("1");
        let receipt = await this.deployer.addNode(id, "127.0.0.1", 1000, 1001, {from: whitelisted});
        truffleAssert.eventEmitted(receipt, newNodeEvent, (ev) => {
            assert.equal(ev.id, id);
            return true
        })
    });

    it("Should send event about enqueued Code", async function() {
        let storageHash = string2Bytes32("abc");
        let receipt = await this.deployer.addCode(storageHash, "bca", 5, {from: whitelisted});

        truffleAssert.eventEmitted(receipt, codeEnqueuedEvent, (ev) => {
            assert.equal(ev.storageHash, storageHash);
            return true
        })
    });

    it("Should throw an error if asking about non-existent cluster", async function() {
        await expectThrow(
            this.deployer.getCluster("abc")
        )
    });

    it("Should deploy code when there are enough nodes", async function() {
        let count = 5;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        let receipts = await addNodes(this.deployer, count, "127.0.0.1", whitelisted);

        truffleAssert.eventEmitted(receipts.pop(), clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            clusterID = ev.clusterID;
            return true;
        });

        let code = await this.deployer.getCluster(clusterID);
        assert.equal(code[0], storageHash);
        assert.equal(code[1], storageReceipt)
    });

    it("Should not form cluster from solvers of same node", async function() {
        await this.deployer.addNode(crypto.randomBytes(16).hexSlice(), "127.0.0.1", 1000, 1002, {from: whitelisted});

        let count = 2;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        let receipt = await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        truffleAssert.eventEmitted(receipt, codeEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt, clusterFormedEvent)
    });

    it("Should reuse node until port range exhausted", async function() {
        let count = 1;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");

        let nodeID = crypto.randomBytes(16).hexSlice();

        await this.deployer.addNode(nodeID, "127.0.0.1", 1000, 1002, {from: whitelisted});

        let receipt1 = await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        truffleAssert.eventNotEmitted(receipt1, codeEnqueuedEvent);
        truffleAssert.eventEmitted(receipt1, clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            assert.equal(ev.solverPorts[0], 1000);
            clusterID1 = ev.clusterID;
            return true;
        });

        let receipt2 = await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        truffleAssert.eventNotEmitted(receipt2, codeEnqueuedEvent);
        truffleAssert.eventEmitted(receipt2, clusterFormedEvent, (ev) => {
            assert.equal(ev.solverAddrs.length, count);
            assert.equal(ev.solverPorts[0], 1001);
            clusterID2 = ev.clusterID;
            return true;
        });

        let receipt3 = await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        truffleAssert.eventEmitted(receipt3, codeEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt3, clusterFormedEvent);

        let nodeClusters = await this.deployer.getNodeClusters(nodeID);

        assert.equal(nodeClusters[0], clusterID1);
        assert.equal(nodeClusters[1], clusterID2)
    });

    it("Should get correct list of clusters and enqueued codes", async function() {
        let count1 = 1;
        let count2 = 2;
        let count3 = 3;
        let count4 = 4;
        let storageHash1 = string2Bytes32("abc");
        let storageHash2 = string2Bytes32("abcd");
        let storageHash3 = string2Bytes32("abcde");
        let storageHash4 = string2Bytes32("abcdef");
        let storageReceipt1 = string2Bytes32("bca");
        let storageReceipt2 = string2Bytes32("dbca");
        let storageReceipt3 = string2Bytes32("edbca");
        let storageReceipt4 = string2Bytes32("fedbca");
        await this.deployer.addCode(storageHash1, storageReceipt1, count1, {from: whitelisted});
        await this.deployer.addCode(storageHash2, storageReceipt2, count2, {from: whitelisted});
        await this.deployer.addCode(storageHash3, storageReceipt3, count3, {from: whitelisted});
        await this.deployer.addCode(storageHash4, storageReceipt4, count4, {from: whitelisted});

        await addNodes(this.deployer, 3, "127.0.0.1", whitelisted);

        let enqueuedCodes = await this.deployer.getEnqueuedCodes();

        assert.equal(enqueuedCodes[0].length, 2);
        assert.equal(enqueuedCodes[0][0], storageHash4);
        assert.equal(enqueuedCodes[0][1], storageHash3);

        assert.equal(enqueuedCodes[1][0], storageReceipt4);
        assert.equal(enqueuedCodes[1][1], storageReceipt3);

        assert.equal(enqueuedCodes[2][0], count4);
        assert.equal(enqueuedCodes[2][1], count3);

        let clustersInfos = await this.deployer.getClustersInfo();
        let clustersNodes = await this.deployer.getClustersNodes();

        assert.equal(clustersInfos[0].length, 2);
        assert.equal(clustersNodes[0].length, 3);

        let nodes = await this.deployer.getReadyNodes();
      assert.equal(nodes[0].length, 3)
    });

    it("Should deploy same code twice", async function() {
        let count = 5;
        let storageHash = string2Bytes32("abc");
        let storageReceipt = string2Bytes32("bca");
        await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});
        await this.deployer.addCode(storageHash, storageReceipt, count, {from: whitelisted});

        let firstCluster = (await addNodes(this.deployer, count, "127.0.0.1", whitelisted)).pop();
        let secondCluster = (await addNodes(this.deployer, count, "127.0.0.1", whitelisted)).pop();

        truffleAssert.eventEmitted(firstCluster, clusterFormedEvent, _ => true);
        truffleAssert.eventEmitted(secondCluster, clusterFormedEvent, _ => true)
    });

    it("Should revert if anyone tries to add code", async function() {
        await expectThrow(
            this.deployer.addCode("storageHash", "storageReceipt", 100)
        );

        await expectThrow(
            this.deployer.addCode("storageHash", "storageReceipt", 100, { from: anyone })
        )
    });

    it("Should revert if anyone tries to add node", async function() {
        await expectThrow(
            this.deployer.addNode("id", "address", 1000, 1001)
        );

        await expectThrow(
            this.deployer.addNode("id", "address", 1000, 1001, { from: anyone })
        )
    })
});
