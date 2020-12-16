import 'mocha';
import Fluence from "../fluence";
import {build} from "../particle";
import { ServiceMultiple } from "../service";
import { registerService } from "../globalState";

describe("== AIR suite", () => {
    it("call local function", async function () {
        Fluence.setLogLevel('debug');

        let service = new ServiceMultiple("console");
        registerService(service);
        service.registerFunction('log', (args: any[]) => {
            console.log(`log: ${args}`);

            return {}
        })

        let client = await Fluence.local();

        let script = `(call %init_peer_id% ("console" "log") ["hello"])`

        // Wrap script into particle, so it can be executed by local WASM runtime
        let particle = await build(client.selfPeerId, script, new Map())

        await client.executeParticle(particle);
    })
})

