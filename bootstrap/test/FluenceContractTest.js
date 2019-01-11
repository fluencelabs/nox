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

contract('Fluence', function ([_, owner, anyone]) {
    beforeEach(async function() {
      this.contract = await FluenceContract.new({ from: owner });
    });

    it("Should send event about new Node", async function() {
        let id = utils.string2Bytes32("1");
        let result = (await utils.addNodesFull(this.contract, 1, "127.0.0.1", anyone, 1)).pop();
        truffleAssert.eventEmitted(result.receipt, utils.newNodeEvent, (ev) => {
            assert.equal(ev.id, result.nodeID);
            return true
        })
    });

    it("Should send event about enqueued App", async function() {
        let result = await utils.addApp(this.contract, 5, anyone);

        truffleAssert.eventEmitted(result.receipt, utils.appEnqueuedEvent, (ev) => {
            assert.equal(ev.storageHash, result.storageHash);
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
        let addApp = await utils.addApp(this.contract, count, anyone);

        let addNodes = await utils.addNodesFull(this.contract, count, "127.0.0.1", anyone);
        let nodeIDs = addNodes.map(r => r.nodeID);
        let receipt = addNodes.pop().receipt;

        let clusterID;

        truffleAssert.eventEmitted(receipt, utils.clusterFormedEvent, (ev) => {
            assert.equal(ev.nodeAddresses.length, count);
            assert.deepEqual(ev.nodeIDs, nodeIDs);
            clusterID = ev.clusterID;
            return true;
        });

        let cluster = await this.contract.getCluster(clusterID);
        assert.equal(cluster[0], addApp.storageHash);
        assert.equal(cluster[1], addApp.storageReceipt);
    });

    it("Should not form cluster from workers of same node", async function() {
        let count = 2;
        
        await utils.addNodes(this.contract, 1, "127.0.0.1", anyone, count)

        let addApp = await utils.addApp(this.contract, count, anyone);

        truffleAssert.eventEmitted(addApp.receipt, utils.appEnqueuedEvent);
        truffleAssert.eventNotEmitted(addApp.receipt, utils.clusterFormedEvent)
    });

    it("Should reuse node until the port range is exhausted", async function() {
        let count = 1;
        let ports = 2;

        let addNodes = await utils.addNodesFull(this.contract, count, "127.0.0.1", anyone, ports);
        let nodeIDs = addNodes.map(r => r.nodeID);

        for (let i = 0; i < ports; i++) {
            let addApp = await utils.addApp(this.contract, count, anyone);

            var clusterID;

            truffleAssert.eventNotEmitted(addApp.receipt, utils.appEnqueuedEvent);
            truffleAssert.eventEmitted(addApp.receipt, utils.clusterFormedEvent, (ev) => {
                assert.equal(ev.nodeAddresses.length, count);
                ev.nodeAddresses.forEach(addr => 
                    assert.equal(utils.bytes32ToString(addr), "127.0.0.1")
                );
                assert.deepEqual(ev.nodeIDs, nodeIDs);
                
                clusterID = ev.clusterID;
                return true;
            });

            nodeIDs.forEach(async id => {
                let nodeClusters = await this.contract.getNodeClusters(id);
                assert.equal(nodeClusters.length, i + 1);
                assert.equal(nodeClusters[i], clusterID);
            });
        }

        let addApp = await utils.addApp(this.contract, count, anyone);
        truffleAssert.eventEmitted(addApp.receipt, utils.appEnqueuedEvent);
        truffleAssert.eventNotEmitted(addApp.receipt, utils.clusterFormedEvent);
    });

    it("Should get correct list of clusters and enqueued codes", async function() {
        let clusterSizes = [1, 2, 3, 4];

        // add 4 apps with different cluster sizes
        let addApps = await Promise.all(clusterSizes.map(size => 
            utils.addApp(this.contract, size, anyone)
        ));

        let allNodes = await utils.addNodesFull(this.contract, 3, "127.0.0.1", anyone, portCount = 2);

        let enqueuedApps = await this.contract.getEnqueuedApps();

        // number of returned fields
        assert.equal(enqueuedApps.length, 6); // storageHashes, storageReceipts, sizes, developerAddresses, pinned, pinnedNodes

        let storageHashes = enqueuedApps[0];
        let storageReceipts = enqueuedApps[1];
        let sizes = enqueuedApps[2];
        let developerAddresses = enqueuedApps[3];
        let pinnedSize = enqueuedApps[4];
        let pinnedNodes = enqueuedApps[5];

        // only two apps were depoyed
        assert.equal(storageHashes.length, 2);
        assert.equal(storageReceipts.length, 2);
        assert.equal(sizes.length, 2);
        assert.equal(developerAddresses.length, 2);

        assert.equal(pinnedSize.length, 2);
        // no pinned nodes in this apps
        assert.equal(pinnedNodes.length, 0);

        // looking for app deployments corresponding to enqueuedApps
        storageHashes.forEach((hash, idx) => {
            let addApp = addApps.find(add => add.storageHash == hash);
            assert.notEqual(addApp, undefined);
            assert.equal(storageReceipts[idx], addApp.storageReceipt);
            assert.equal(sizes[idx], addApp.clusterSize);
            assert.equal(developerAddresses[idx], anyone);
            assert.equal(pinnedSize[idx], 0);
        });

        let nodesIds = await this.contract.getNodesIds();
        assert.equal(nodesIds.length, 3);
        assert.equal(nodesIds[0], allNodes[0].nodeID);
        assert.equal(nodesIds[1], allNodes[1].nodeID);

        let clustersIds = await this.contract.getClustersIds();
        assert.equal(clustersIds.length, 2);
    });

    it("Should deploy same code twice", async function() {
        let count = 5;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: anyone});
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: anyone});

        let firstCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", anyone, portCount = 1)).pop();
        let secondCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", anyone, portCount = 1)).pop();

        truffleAssert.eventEmitted(firstCluster, utils.clusterFormedEvent, _ => true);
        truffleAssert.eventEmitted(secondCluster, utils.clusterFormedEvent, _ => true)
    });
});
