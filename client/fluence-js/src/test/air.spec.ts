import 'mocha';
import Fluence from "../fluence";
import {build} from "../particle";
import {ServiceMultiple} from "../service";
import {registerService} from "../globalState";
import {expect} from "chai";

function registerPromiseService<T>(serviceId: string, fnName: string, f: (args: any[]) => T): Promise<T> {
    let service = new ServiceMultiple(serviceId);
    registerService(service);

    return new Promise((resolve, reject) => {
        service.registerFunction(fnName, (args: any[]) => {
            resolve(f(args))

            return {result: f(args)}
        })
    })
}

describe("== AIR suite", () => {

    it("check init_peer_id", async function () {
        let serviceId = "init_peer"
        let fnName = "id"
        let checkPromise = registerPromiseService(serviceId, fnName, (args) => args[0])

        let client = await Fluence.local();

        let script = `(call %init_peer_id% ("${serviceId}" "${fnName}") [%init_peer_id%])`

        let particle = await build(client.selfPeerId, script, new Map())

        await client.executeParticle(particle);

        expect(await checkPromise).to.be.equal(client.selfPeerIdStr)
    })

    it("call local function", async function () {
        let serviceId = "console"
        let fnName = "log"
        let checkPromise = registerPromiseService(serviceId, fnName, (args) => args[0])

        let client = await Fluence.local();

        let arg = "hello"
        let script = `(call %init_peer_id% ("${serviceId}" "${fnName}") ["${arg}"])`

        // Wrap script into particle, so it can be executed by local WASM runtime
        let particle = await build(client.selfPeerId, script, new Map())

        await client.executeParticle(particle);

        expect(await checkPromise).to.be.equal(arg)
    })

    it("check particle arguments", async function () {
        let serviceId = "check"
        let fnName = "args"
        let checkPromise = registerPromiseService(serviceId, fnName, (args) => args[0])

        let client = await Fluence.local();

        let arg = "arg1"
        let value = "hello"
        let script = `(call %init_peer_id% ("${serviceId}" "${fnName}") [${arg}])`


        let data = new Map()
        data.set("arg1", value)
        let particle = await build(client.selfPeerId, script, data)

        await client.executeParticle(particle);

        expect(await checkPromise).to.be.equal(value)
    })

    it("check chain of services work properly", async function () {
        this.timeout(5000);
        let serviceId1 = "check1"
        let fnName1 = "fn1"
        let checkPromise1 = registerPromiseService(serviceId1, fnName1, (args) => args[0])

        let serviceId2 = "check2"
        let fnName2 = "fn2"
        let checkPromise2 = registerPromiseService(serviceId2, fnName2, (args) => args[0])

        let serviceId3 = "check3"
        let fnName3 = "fn3"
        let checkPromise3 = registerPromiseService(serviceId3, fnName3, (args) => args)

        let client = await Fluence.local();

        let arg1 = "arg1"
        let arg2 = "arg2"

        // language=Clojure
        let script = `(seq
                       (seq
                        (call %init_peer_id% ("${serviceId1}" "${fnName1}") ["${arg1}"] result1)
                        (call %init_peer_id% ("${serviceId2}" "${fnName2}") ["${arg2}"] result2))
                       (call %init_peer_id% ("${serviceId3}" "${fnName3}") [result1 result2]))
        `

        let particle = await build(client.selfPeerId, script, new Map())

        await client.executeParticle(particle);

        expect(await checkPromise1).to.be.equal(arg1)
        expect(await checkPromise2).to.be.equal(arg2)

        expect(await checkPromise3).to.be.deep.equal([{result: arg1}, {result: arg2}])
    })
})

