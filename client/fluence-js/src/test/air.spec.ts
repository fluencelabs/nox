import 'mocha';
import Fluence from "../fluence";
import {build} from "../particle";
import { ServiceMultiple } from "../service";

type Exports = { malloc: (a: any) => void };
type ImportObject = {
    module: {
        func: (arg0: any) => void;
    }
};

interface Instance {
    readonly importObject: ImportObject;
}

class HostImportsConfig {
    exports: Exports | undefined;
    create: () => ImportObject;

    constructor(create: (cfg: HostImportsConfig) => ImportObject) {
        this.exports = undefined;
        this.create = () => create(this)
    }
}

async function newInstance(hostImports: HostImportsConfig): Promise<Instance> {
    let instance = await new Promise<Instance>((resolve, _) => {
        let i: Instance = {
            importObject: hostImports.create(),
        };
        resolve(i);
    });

    hostImports.exports = {
        malloc: (a: any) => {
            console.log(`malloc ${a}`);
        }
    }

    return instance;
}

describe("AIR", () => {
    it("call local function", async function () {
        let service = new ServiceMultiple("console");
        service.registerFunction('log', (args: any[]) => {
            console.log(`log: ${args}`);

            return {}
        })

        let client = await Fluence.local();

        let script = `(call %init_peer_id% ("console" "log") ["hello"])`

        // Wrap script into particle, so it can be executed by local WASM runtime
        let particle = await build(client.selfPeerId, script, new Map())

        await client.sendParticle(particle);
    })

    it("test lambda reference", async function() {
        const hostImports = new HostImportsConfig((cfg) => {
            return {
                module: {
                    func: (arg0: any) => {
                        cfg.exports.malloc(arg0)
                    }
                }
            }
        });

        let instance = await newInstance(hostImports);

        instance.importObject.module.func(123);
    })
})

