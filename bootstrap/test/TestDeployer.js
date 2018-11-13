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

var Deployer = artifacts.require("./Deployer.sol")
const truffleAssert = require('truffle-assertions')
const assert = require("chai").assert
const crypto = require("crypto")
const should = require('chai').should();
const { expectThrow } = require('openzeppelin-solidity/test/helpers/expectThrow');

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
    })

    it("Should send event about new Node", async function() {
        let id = string2Bytes32("1")
        let receipt = await this.deployer.addNode(id, "127.0.0.1", 1000, 1001, { from: whitelisted })
        truffleAssert.eventEmitted(receipt, 'NewNode', (ev) => {
            assert.equal(ev.id, id)
            return true
        })
    })

    it("Should send event about enqueued Code", async function() {
        let storageHash  = string2Bytes32("abc")
        let receipt = await this.deployer.addCode(storageHash, "bca", 5, { from: whitelisted })

        truffleAssert.eventEmitted(receipt, 'CodeEnqueued', (ev) => {
            assert.equal(ev.storageHash, storageHash)
            return true
        })
    })

    it("Should throw an error if asking about non-existent cluster", async function() {
        await expectThrow(
            this.deployer.getCluster("abc")
        )
    })

    it("Should deploy code when there is enough solvers", async function() {
        let count = 5
        let storageHash = string2Bytes32("abc")
        let storageReceipt = string2Bytes32("bca")
        await this.deployer.addCode(storageHash, storageReceipt, count, { from: whitelisted })

        let receipts = await addNodes(this.deployer, count, "127.0.0.1", whitelisted)

        truffleAssert.eventEmitted(receipts.pop(), 'ClusterFormed', (ev) => {
            assert.equal(ev.solverIDs.length, count)
            clusterID = ev.clusterID
            return true;
        })

        let code = await this.deployer.getCluster(clusterID)
        assert.equal(code[0], storageHash)
        assert.equal(code[1], storageReceipt)
    })

    it("Should deploy same code twice", async function() {
        let count = 5
        let storageHash = string2Bytes32("abc")
        let storageReceipt = string2Bytes32("bca")
        await this.deployer.addCode(storageHash, storageReceipt, count, { from: whitelisted })
        await this.deployer.addCode(storageHash, storageReceipt, count, { from: whitelisted })
        
        let firstCluster = (await addNodes(this.deployer, count, "127.0.0.1", whitelisted)).pop()
        let secondCluster = (await addNodes(this.deployer, count, "127.0.0.1", whitelisted)).pop()

        truffleAssert.eventEmitted(firstCluster, 'ClusterFormed', _ => true)
        truffleAssert.eventEmitted(secondCluster, 'ClusterFormed', _ => true)
    })

    it("Should revert if anyone tries to add code", async function() {
        await expectThrow(
            this.deployer.addCode("storageHash", "storageReceipt", 100)
        )

        await expectThrow(
            this.deployer.addCode("storageHash", "storageReceipt", 100, { from: anyone })
        )
    })

    it("Should revert if anyone tries to add node", async function() {
        await expectThrow(
            this.deployer.addNode("id", "address", 1000, 1001)
        )

        await expectThrow(
            this.deployer.addNode("id", "address", 1000, 1001, { from: anyone })
        )
    })
})
