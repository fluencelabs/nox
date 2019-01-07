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
const utils = require("./Utils.js");
const truffleAssert = require('truffle-assertions');
const assert = require("chai").assert;
const { expectThrow } = require('openzeppelin-solidity/test/helpers/expectThrow');

contract('Fluence (pinning)', function ([_, owner, whitelisted, anyone]) {
    beforeEach(async function() {
      global.contract = await FluenceContract.new({ from: owner });
    });

    async function addPinnedNodes(count, ports = 1) {
        return utils.addPinnedNodes(global.contract, count, "127.0.0.1", owner, ports);
    }

    async function addNodes(count, ports = 1) {
        return utils.addNodes(global.contract, count, "127.0.0.1", anyone, ports);
    }

    async function addApp(count, ids = []) {
        return await utils.addApp(global.contract, count, owner, ids);
    }
    
    it("Should form cluster of pinned nodes and ignore other nodes", async function() {
        let cluster = 5;
        // add some nodes before pinned so order doesn't matter
        await addNodes(cluster);
        
        var result = await addPinnedNodes(3);
        
        // add some nodes in the middle of pinned nodes so order doesn't matter
        await addNodes(1);
        
        // add pinned nodes & concatenate results
        result = [...(await addPinnedNodes(cluster - 3)), ...result];

        // add some nodes after pinned so order doesn't matter
        await addNodes(cluster);
        
        let pinnedNodes = result.map(r => r.nodeID);
        let receipt = (await addApp(cluster, pinnedNodes)).receipt;
        
        var clusterID;
        truffleAssert.eventEmitted(receipt, utils.clusterFormedEvent, (ev) => {
            assert.deepEqual(ev.nodeIDs, pinnedNodes);
            clusterID = ev.clusterID;
            return true
        });

        let clusterInfo = await global.contract.getCluster(clusterID);
        let clusterSize = clusterInfo[2];
        let appPinToNodes = clusterInfo[4];
        let nodeIDs = clusterInfo[6];

        assert.equal(clusterSize, cluster);
        assert.deepEqual(appPinToNodes, pinnedNodes);
        assert.deepEqual(nodeIDs, pinnedNodes);
    });

    it("Should use both public & private nodes for pinned app", async function() {
        let count = 5;
        let pinnedCount = 2;

        let pinned = await addPinnedNodes(pinnedCount);
        let pinnedNodeIDs = pinned.map(r => r.nodeID);
        let add = await addApp(count, pinnedNodeIDs);

        // Cluster isn't formed yet
        truffleAssert.eventNotEmitted(add.receipt, utils.clusterFormedEvent, _ => true);

        // App is enqueued
        truffleAssert.eventEmitted(add.receipt, utils.appEnqueuedEvent, ev => {
            assert.equal(ev.storageHash, add.storageHash);
            return true
        })

        // Add remaining public nodes
        let result = await addNodes(count - pinnedCount);
        let receipt = result.pop();

        // Cluster is formed
        truffleAssert.eventEmitted(receipt, utils.clusterFormedEvent, ev => {
            pinnedNodeIDs.forEach(id => 
                assert.include(ev.nodeIDs, id)
            );
            return true
        });
    });

    it("Should enqueue several apps & deploy them in a single tx", async function() {
        // Number of nodes required by apps
        let count = 6;

        // Number of apps
        let apps = 5;
        
        // Adding pinned nodes, apps will require 1 more node to form a cluster
        let results = await addPinnedNodes(count - 1, ports = apps);
        let pinnedNodeIDs = results.map(r => r.nodeID);

        // Adding apps
        for (let i = 0; i < apps; i++) {
            let result = await addApp(count, pinnedNodeIDs);

            // App should be enqueued
            truffleAssert.eventEmitted(result.receipt, utils.appEnqueuedEvent, ev => {
                assert.equal(ev.storageHash, result.storageHash);
                return true
            })
        }

        // adding 1 node that can host `apps` apps. should trigger ClusterFormed
        let receipts = await addNodes(1, ports = apps);

        var eventCount = 0;
        truffleAssert.eventEmitted(receipts.pop(), utils.clusterFormedEvent, ev => {
            assert.equal(ev.nodeIDs.length, count);
            // should be deployed on all pinned nodes + one public node (not checked here)
            pinnedNodeIDs.forEach(id => 
                assert.include(ev.nodeIDs, id)
            )
            eventCount += 1;
            return true;
        });

        assert.equal(eventCount, apps);
    });

    it("Should revert on pinToNodes > clusterSize", async function() {
        await expectThrow(
            addApp(5, Array(10).fill(0)),
            "number of pinTo nodes should be less or equal to the desired clusterSize"
        );
    });

    it("Should revert on pinning to non-registered node", async function() {
        await expectThrow(
            addApp(5, ["non-existent-node-id"]),
            "Can pin only to registered nodes"
        );
    });

    it("Should revert on pinning to non-owned node", async function() {
        let result = await utils.addPinnedNodes(global.contract, 1, "127.0.0.1", anyone, 1);
        let nodeID = result.pop().nodeID;
        await expectThrow(
            addApp(5, [nodeID]),
            "Can pin only to nodes you own"
        );
    });
});
