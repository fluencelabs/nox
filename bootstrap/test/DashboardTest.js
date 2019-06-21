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
var DashboardContract = artifacts.require("./Dashboard.sol");
const utils = require("./Utils.js");
const assert = require("chai").assert;

contract('Dashboard', function ([_, owner, anyone, other]) {
    beforeEach(async function() {
        global.contract = await FluenceContract.new({ from: owner });
        global.dashboard = await DashboardContract.new(contract.address, { from: owner })
    });

    async function addNodes(count, ports = 1) {
        return utils.addNodesFull(global.contract, count, "127.0.0.1", anyone, ports);
    }

    async function addApp(count, ids = []) {
        return await utils.addApp(global.contract, count, anyone, ids);
    }

    it("Get apps (dashboard)", async function() {
        await addNodes(3, 3);
        let app1 = await addApp(3);
        let app2 = await addApp(3);
        let app3 = await addApp(3);
        let result = await global.dashboard.getApps();
        assert.equal(result[0].length, 3);
        assert.equal(result[1].length, 3);
        assert.equal(result[2].length, 3);

        assert.equal(result[1][0], app1.storageHash);
        assert.equal(result[1][1], app2.storageHash);
        assert.equal(result[1][2], app3.storageHash);

        assert.equal(result[2][0], anyone);
        assert.equal(result[2][1], anyone);
        assert.equal(result[2][2], anyone);
    });

    it("Get nodes (dashboard)", async function () {
        let count = 3;
        let add = await addNodes(count, count);

        let result = await global.dashboard.getNodes();
        let nodeIDs = result[0];
        let owners = result[1];
        let capacity = result[2];
        let privacy = result[3];

        assert.equal(nodeIDs.length, count);
        assert.equal(owners.length, count);
        assert.equal(capacity.length, count);
        assert.equal(privacy.length, count);

        owners.forEach(o => assert.equal(o, anyone));
        capacity.forEach(c => assert.equal(c, count));
        privacy.forEach(p => assert.equal(p, false));

        for (let i = 0; i < count; i++) {
            assert.equal(add[i].nodeID, nodeIDs[i]);
        }
    });
});
