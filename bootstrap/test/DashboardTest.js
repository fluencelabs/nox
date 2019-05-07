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
        await addApp(3);
        await addApp(3);
        await addApp(3);
        var result = await global.dashboard.getApps();
        assert.equal(result[0].length, 3);
        assert.equal(result[1].length, 3);
    });
});