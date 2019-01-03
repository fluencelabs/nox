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
const utils = require('./Utils.js');
const truffleAssert = require('truffle-assertions');
const assert = require("chai").assert;
const crypto = require("crypto");
const { expectThrow } = require('openzeppelin-solidity/test/helpers/expectThrow');

contract('Fluence', function ([_, owner, whitelisted, anyone]) {
    beforeEach(async function() {
      this.contract = await FluenceContract.new({ from: owner });
    });

    it("Should send event about new Node", async function() {
        let id = utils.string2Bytes32("1");
        let receipt = await this.contract.addNode(id, "127.0.0.1", 1000, 1001, false, {from: whitelisted});
        truffleAssert.eventEmitted(receipt, utils.newNodeEvent, (ev) => {
            assert.equal(ev.id, id);
            return true
        })
    });

    it("Should send event about enqueued App", async function() {
        let storageHash = utils.string2Bytes32("abc");
        let receipt = await this.contract.addApp(storageHash, "bca", 5, [], {from: whitelisted});

        truffleAssert.eventEmitted(receipt, utils.appEnqueuedEvent, (ev) => {
            assert.equal(ev.storageHash, storageHash);
            return true
        })
    });

    it("Should throw an error if asking about non-existent cluster", async function() {
        await expectThrow(
            this.contract.getCluster("abc")
        )
    });

    it("Should deploy an app when there are enough nodes", async function() {
        let count = 5;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});

        let receipts = await utils.addNodes(this.contract, count, "127.0.0.1", whitelisted);

        let clusterID;

        truffleAssert.eventEmitted(receipts.pop(), utils.clusterFormedEvent, (ev) => {
            assert.equal(ev.nodeAddresses.length, count);
            clusterID = ev.clusterID;
            return true;
        });

        let cluster = await this.contract.getCluster(clusterID);
        assert.equal(cluster[0], storageHash);
        assert.equal(cluster[1], storageReceipt);
    });

    it("Should not form cluster from workers of same node", async function() {
        await this.contract.addNode(crypto.randomBytes(16).hexSlice(), "127.0.0.1", 1000, 1002, false, {from: whitelisted});

        let count = 2;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");
        let receipt = await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});

        truffleAssert.eventEmitted(receipt, utils.appEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt, utils.clusterFormedEvent)
    });

    it("Should reuse node until the port range is exhausted", async function() {
        let count = 1;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");

        let nodeID = crypto.randomBytes(16).hexSlice();

        await this.contract.addNode(nodeID, "127.0.0.1", 1000, 1001, false, {from: whitelisted});

        let receipt1 = await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});

        truffleAssert.eventNotEmitted(receipt1, utils.appEnqueuedEvent);
        truffleAssert.eventEmitted(receipt1, utils.clusterFormedEvent, (ev) => {
            assert.equal(ev.nodeAddresses.length, count);
            assert.equal(ev.ports[0], 1000);
            clusterID1 = ev.clusterID;
            return true;
        });

        let receipt2 = await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});
        truffleAssert.eventNotEmitted(receipt2, utils.appEnqueuedEvent);
        truffleAssert.eventEmitted(receipt2, utils.clusterFormedEvent, (ev) => {
            assert.equal(ev.nodeAddresses.length, count);
            assert.equal(ev.ports[0], 1001);
            clusterID2 = ev.clusterID;
            return true;
        });

        let receipt3 = await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});
        truffleAssert.eventEmitted(receipt3, utils.appEnqueuedEvent);
        truffleAssert.eventNotEmitted(receipt3, utils.clusterFormedEvent);

        let nodeClusters = await this.contract.getNodeClusters(nodeID);

        assert.equal(nodeClusters[0], clusterID1);
        assert.equal(nodeClusters[1], clusterID2);
    });

    it("Should get correct list of clusters and enqueued codes", async function() {
        let [count1, count2, count3, count4] = [1, 2, 3, 4];

        let [storageHash1, storageHash2, storageHash3, storageHash4] =
            ["abc","abcd","abcde","abcdef"].map(s => utils.string2Bytes32(s));

        let [storageReceipt1, storageReceipt2, storageReceipt3, storageReceipt4] =
            ["xyz","xyzd","xyzde","xyzdef"].map(s => utils.string2Bytes32(s));

        await this.contract.addApp(storageHash1, storageReceipt1, count1, [], {from: whitelisted});
        await this.contract.addApp(storageHash2, storageReceipt2, count2, [], {from: whitelisted});
        await this.contract.addApp(storageHash3, storageReceipt3, count3, [], {from: whitelisted});
        await this.contract.addApp(storageHash4, storageReceipt4, count4, [], {from: whitelisted});

        await utils.addNodes(this.contract, 3, "127.0.0.1", whitelisted, portCount = 2);

        let enqueuedApps = await this.contract.getEnqueuedApps();

        assert.equal(enqueuedApps.length, 4); // storageHashes, storageReceipts, clusterSizes, developerAddresses, pinned, pinnedNodes

        let storageHashes = enqueuedApps[0];
        let storageReceipts = enqueuedApps[1];
        let clusterSizes = enqueuedApps[2];
        let developerAddresses = enqueuedApps[3];

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

        // FIXME
        // let clustersInfos = await this.contract.getClustersInfo();
        // let clustersNodes = await this.contract.getClustersNodes();
        //
        // assert.equal(clustersInfos[0].length, 2);
        // assert.equal(clustersNodes[0].length, 3);
        //
        // let nodes = await this.contract.getReadyNodes();
        // assert.equal(nodes[0].length, 3)
    });

    it("Should deploy same code twice", async function() {
        let count = 5;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: whitelisted});

        let firstCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", whitelisted, portCount = 1)).pop();
        let secondCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", whitelisted, portCount = 1)).pop();

        truffleAssert.eventEmitted(firstCluster, utils.clusterFormedEvent, _ => true);
        truffleAssert.eventEmitted(secondCluster, utils.clusterFormedEvent, _ => true)
    });
});
