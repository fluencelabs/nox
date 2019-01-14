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

contract('Fluence (app deletion)', function ([_, owner, whitelisted, anyone]) {
    beforeEach(async function() {
        global.contract = await FluenceContract.new({ from: owner });
    });

    async function addNodes(count, ports = 1) {
        return utils.addNodes(global.contract, count, "127.0.0.1", anyone, ports);
    }

    async function addApp(count, ids = []) {
        return await utils.addApp(global.contract, count, owner, ids);
    }

    it("Remove enqueued app", async function() {
        let add = await addApp(1);
        var appID;
        truffleAssert.eventEmitted(add.receipt, utils.appEnqueuedEvent, ev => {
            appID = ev.appID;
            return true;
        });

        let enqueuedApps = await global.contract.getEnqueuedApps();
        let appIDs = enqueuedApps[1];
        let app = appIDs.find(app => app == appID);
        assert.notEqual(app, undefined);

        // only app owner can delete app
        await expectThrow(global.contract.deleteApp(appID, 0, { from: anyone }));

        let deleteApp = await global.contract.deleteApp(appID, 0, { from: owner });
        truffleAssert.eventEmitted(deleteApp, utils.appDeletedEvent, ev => {
            assert.equal(ev.appID, appID);
            assert.equal(ev.clusterID, 0);

            return true;
        });
    });

    it("Remove deployed app", async function() {
        let add = await addApp(5);
        var appID;
        truffleAssert.eventEmitted(add.receipt, utils.appEnqueuedEvent, ev => {
            appID = ev.appID;
            return true;
        });

        let nodesReceipts = await addNodes(5);
        var clusterID;
        truffleAssert.eventEmitted(nodesReceipts.pop(), utils.clusterFormedEvent, ev => {
            assert.equal(ev.appID, appID);
            clusterID = ev.clusterID;
            return true;
        });

        let cluster = await global.contract.getCluster(clusterID);
        assert.equal(cluster[5], appID);

        // only app owner can delete app
        await expectThrow(global.contract.deleteApp(appID, 0, { from: anyone }));

        // app was deployed, can't delete without clusterID
        await expectThrow(global.contract.deleteApp(appID, 0, { from: owner }));

        let deleteApp = await global.contract.deleteApp(appID, clusterID, { from: owner });
        truffleAssert.eventEmitted(deleteApp, utils.appDeletedEvent, ev => {
            assert.equal(ev.appID, appID);
            assert.equal(ev.clusterID, clusterID);

            return true;
        });

        await expectThrow(global.contract.getCluster(clusterID));
    });
});