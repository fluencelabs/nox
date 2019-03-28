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
const { shouldFail } = require('openzeppelin-test-helpers');

contract('Fluence (node deletion)', function ([_, owner, anyone, other]) {
    beforeEach(async function() {
        global.contract = await FluenceContract.new({ from: owner });
    });

    async function addNodes(count, ports = 1) {
        return utils.addNodes(global.contract, count, "127.0.0.1", anyone, ports);
    }

    async function addApp(count, ids = []) {
        return await utils.addApp(global.contract, count, owner, ids);
    }


    it("Remove enqueued node", async function() {
        let add = await addNodes(1);
        let nodeID;
        truffleAssert.eventEmitted(add.pop(), utils.newNodeEvent, ev => {
            nodeID = ev.id;
            return true;
        });

        let receipt = await global.contract.deleteNode(nodeID, { from: anyone });
        truffleAssert.eventEmitted(receipt, utils.nodeDeletedEvent, ev => {
            return ev.id === nodeID;
        });
    });

    it("Contract owner should be able to remove node", async function() {
        let add = await addNodes(1);
        let nodeID;
        truffleAssert.eventEmitted(add.pop(), utils.newNodeEvent, ev => {
            nodeID = ev.id;
            return true;
        });

        await shouldFail.reverting(
            global.contract.deleteNode(nodeID, { from: other }),
            "error deleting node: you must own node to delete it"
        );

        await shouldFail.reverting(
            global.contract.deleteNode("wrongNodeId", { from: other }),
            "error deleting node: node not found"
        );

        let receipt = await global.contract.deleteNode(nodeID, { from: owner });
        truffleAssert.eventEmitted(receipt, utils.nodeDeletedEvent, ev => {
            return ev.id === nodeID;
        });
    });

    it("Remove nodes with deployed app", async function() {
        let count = 4;
        let adds = await addNodes(count);

        var nodeIDs = [];
        adds.forEach(add => {
            truffleAssert.eventEmitted(add, utils.newNodeEvent, ev => {
                nodeIDs.push(ev.id);
                return true;
            });
        });

        assert.equal(nodeIDs.length, count);

        await addApp(count);

        let receipts = await Promise.all(nodeIDs.map(async nodeID => {
            let receipt = await global.contract.deleteNode(nodeID, { from: anyone });
            truffleAssert.eventEmitted(receipt, utils.nodeDeletedEvent, ev => {
                return ev.id === nodeID;
            });
            return receipt;
        }));

        truffleAssert.eventEmitted(receipts.pop(), utils.appDeletedEvent);
    });
});