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
const { shouldFail, expectEvent } = require('openzeppelin-test-helpers');

contract('Fluence (app deletion)', function ([_, owner, anyone, other]) {
    beforeEach(async function() {
        global.contract = await FluenceContract.new({ from: owner });
    });

    async function addNodes(count, ports = 1) {
        return utils.addNodesFull(global.contract, count, "127.0.0.1", anyone, ports);
    }

    async function addApp(count, ids = []) {
        return await utils.addApp(global.contract, count, anyone, ids);
    }

    it("Remove enqueued app", async function() {
        let add = await addApp(1);
        let appID = expectEvent.inLogs(add.logs, utils.appEnqueuedEvent).args.appID;

        let app = await global.contract.getApp(appID);
        assert.notEqual(app, undefined);
        let storageHash = app[0];
        assert.equal(storageHash, add.storageHash);

        // only app owner can delete app
        await shouldFail.reverting(global.contract.dequeueApp(appID, { from: other }));

        let dequeueApp = await global.contract.dequeueApp(appID, { from: anyone });
        expectEvent.inLogs(dequeueApp.logs, utils.appDequeuedEvent, { appID: appID });

        await shouldFail.reverting(global.contract.getApp(appID)); // throws on non existing app

        let appIDs = await global.contract.getAppIDs();
        assert.equal(0, appIDs.length);
    });

    it("Remove deployed app", async function() {
        let add = await addApp(5);
        let appID = expectEvent.inLogs(add.logs, utils.appEnqueuedEvent).args.appID;

        let nodesResponse = await addNodes(5);

        let nodeIds = nodesResponse.map(r => r.nodeID);

        expectEvent.inLogs(nodesResponse.pop().logs, utils.appDeployedEvent, { appID: appID });

        let cluster = await global.contract.getApp(appID);
        let storageHash = cluster[0];
        assert.equal(storageHash, add.storageHash);

        // only app owner can delete app
        await shouldFail.reverting(global.contract.deleteApp(appID, { from: other }));

        // can't delete with wrong clusterID
        await shouldFail.reverting(global.contract.deleteApp(0, { from: anyone }));

        let deleteApp = await global.contract.deleteApp(appID, { from: anyone });
        expectEvent.inLogs(deleteApp.logs, utils.appDeletedEvent, { appID: appID });

        await shouldFail.reverting(global.contract.getApp(appID));

        assert.equal(nodeIds.length, 5);

        let nodes = await Promise.all(nodeIds.map((id) => {
            return global.contract.getNode(id);
        }));

        nodes.forEach((n) => {
            assert.equal(n[5].length, 0);
        })
    });

    it("Contract owner should be able to dequeue app", async function() {
        let add = await addApp(1);

        let appID = expectEvent.inLogs(add.logs, utils.appEnqueuedEvent).args.appID;

        let dequeueApp = await global.contract.dequeueApp(appID, { from: owner });
        expectEvent.inLogs(dequeueApp.logs, utils.appDequeuedEvent, { appID: appID });
    });

    it("Contract owner should be able to delete app", async function() {
        let add = await addApp(5);
        let appID = expectEvent.inLogs(add.logs, utils.appEnqueuedEvent).args.appID;

        await addNodes(5);

        let deleteApp = await global.contract.deleteApp(appID, { from: owner });
        expectEvent.inLogs(deleteApp.logs, utils.appDeletedEvent, { appID: appID });
    });

    it("Enqueued app should be deployed after capacity increase", async function() {
        await addNodes(5);
        let add = await addApp(3);
        let deployedAppId = expectEvent.inLogs(add.logs, utils.appDeployedEvent).args.appID;

        add = await addApp(3);
        let enqueuedAppId = expectEvent.inLogs(add.logs, utils.appEnqueuedEvent).args.appID;

        let deleteApp = await global.contract.deleteApp(deployedAppId, { from: owner });

        expectEvent.inLogs(deleteApp.logs, utils.appDeletedEvent, { appID: deployedAppId });
        expectEvent.inLogs(deleteApp.logs, utils.appDeployedEvent, { appID: enqueuedAppId });
    });
});
