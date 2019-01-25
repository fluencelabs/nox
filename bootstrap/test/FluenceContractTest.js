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
const { expectThrow } = require('openzeppelin-solidity/test/helpers/expectThrow');

contract('Fluence', function ([_, owner, anyone]) {
    beforeEach(async function() {
      this.contract = await FluenceContract.new({ from: owner });
    });

    it("Should send event about new Node", async function() {
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
            this.contract.getApp("abc")
        )
    });

    it("Should deploy an app when there are enough nodes", async function() {
        let count = 5;
        let addApp = await utils.addApp(this.contract, count, anyone);
        let appID;
        truffleAssert.eventEmitted(addApp.receipt, utils.appEnqueuedEvent, ev => {
            appID = ev.appID;
            return true;
        });
        assert.notEqual(appID, undefined);

        let addNodes = await utils.addNodesFull(this.contract, count, "127.0.0.1", anyone);
        let nodeIDs = addNodes.map(r => r.nodeID);
        let receipt = addNodes.pop().receipt;

        truffleAssert.eventEmitted(receipt, utils.appDeployedEvent, (ev) => {
            assert.equal(ev.nodeAddresses.length, count);
            assert.deepEqual(ev.nodeIDs, nodeIDs);
            assert.equal(ev.appID, appID);
            return true;
        });

        let cluster = await this.contract.getApp(appID);
        assert.equal(cluster[0], addApp.storageHash);
        assert.equal(cluster[1], addApp.storageReceipt);
    });

    it("Should not form cluster from workers of same node", async function() {
        let count = 2;
        
        await utils.addNodes(this.contract, 1, "127.0.0.1", anyone, count);

        let addApp = await utils.addApp(this.contract, count, anyone);

        truffleAssert.eventEmitted(addApp.receipt, utils.appEnqueuedEvent);
        truffleAssert.eventNotEmitted(addApp.receipt, utils.appDeployedEvent)
    });

    it("Should reuse node until the port range is exhausted", async function() {
        let count = 1;
        let ports = 2;

        let addNodes = await utils.addNodesFull(this.contract, count, "127.0.0.1", anyone, ports);
        let nodeIDs = addNodes.map(r => r.nodeID);

        for (let i = 0; i < ports; i++) {
            let addApp = await utils.addApp(this.contract, count, anyone);

            let appID;

            truffleAssert.eventNotEmitted(addApp.receipt, utils.appEnqueuedEvent);
            truffleAssert.eventEmitted(addApp.receipt, utils.appDeployedEvent, (ev) => {
                assert.equal(ev.nodeAddresses.length, count);
                ev.nodeAddresses.forEach(addr => 
                    assert.equal(utils.bytes32ToString(addr), "127.0.0.1")
                );
                assert.deepEqual(ev.nodeIDs, nodeIDs);
                
                appID = ev.appID;
                return true;
            });

            nodeIDs.forEach(async id => {
                let nodeApps = await this.contract.getNodeApps(id);
                assert.equal(nodeApps.length, i + 1);
                assert.equal(nodeApps[i], appID);
            });
        }

        let addApp = await utils.addApp(this.contract, count, anyone);
        var appID;
        truffleAssert.eventEmitted(addApp.receipt, utils.appEnqueuedEvent, (ev) => {
            appID = ev.appID;
            return true;
        });

        // check app with that ID is in enqueued apps list
        let appIDs = await this.contract.getAppIDs();
        let enqueuedApp = appIDs.find(app => app === appID);
        assert.notEqual(enqueuedApp, undefined);
        truffleAssert.eventNotEmitted(addApp.receipt, utils.appDeployedEvent);
    });

    it("Should get correct list of clusters and enqueued codes", async function() {
        let clusterSizes = [1, 2, 3, 4];

        // add 4 apps with different cluster sizes
        let addApps = await Promise.all(clusterSizes.map(size => 
            utils.addApp(this.contract, size, anyone)
        ));

        let allNodes = await utils.addNodesFull(this.contract, 3, "127.0.0.1", anyone, portCount = 2);

        let appIDs = await this.contract.getAppIDs();
        assert.equal(appIDs.length, 4);

        let deployedApps = appIDs.filter(async (appID) => {
            let app = await this.contract.getApp(appID);

            let storageHash = app[0];
            let storageReceipt = app[1];
            let clusterSize = app[2];
            let owner = app[3];
            let pin_to = app[4];

            let addApp = addApps.find(add => add.storageHash === storageHash);
            assert.notEqual(addApp, undefined);

            assert.equal(addApp.storageHash, storageHash);
            assert.equal(addApp.storageReceipt, storageReceipt);
            assert.equal(addApp.clusterSize, clusterSize);
            assert.equal(anyone, owner);
            assert.equal(0, pin_to.length);

            let genesis = app[5];
            let nodeIDs = app[6];
            let ports = app[7];

            return genesis > 0 && nodeIDs.length > 0 && ports.length > 0;
        });

        assert(deployedApps.length, 2);

        let nodesIds = await this.contract.getNodesIds();
        assert.equal(nodesIds.length, 3);
        assert.equal(nodesIds[0], allNodes[0].nodeID);
        assert.equal(nodesIds[1], allNodes[1].nodeID);
    });

    it("Should deploy same code twice", async function() {
        let count = 5;
        let storageHash = utils.string2Bytes32("abc");
        let storageReceipt = utils.string2Bytes32("bca");
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: anyone});
        await this.contract.addApp(storageHash, storageReceipt, count, [], {from: anyone});

        let firstCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", anyone, portCount = 1)).pop();
        let secondCluster = (await utils.addNodes(this.contract, count, "127.0.0.1", anyone, portCount = 1)).pop();

        truffleAssert.eventEmitted(firstCluster, utils.appDeployedEvent, () => true);
        truffleAssert.eventEmitted(secondCluster, utils.appDeployedEvent, () => true)
    });
});
