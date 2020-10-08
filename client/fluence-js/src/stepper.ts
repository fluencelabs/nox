import {wasmBs64} from "../wasmBs64";
import {toByteArray} from "base64-js";
import * as aqua from "./aqua"

import {callService} from "./callService";
import {getInt32Memory0, getStringFromWasm0, passStringToWasm0, WASM_VECTOR_LEN} from "./aqua";
import PeerId from "peer-id";

export type Stepper = (init_user_id: string, script: string, data: string) => string

export async function instantiateStepper(pid: PeerId): Promise<Stepper> {
    // Fetch our Wasm File
    const arr = toByteArray(wasmBs64)

    let wasm: any = undefined;

    const importObject = {
        "./index_bg.js": { __wbg_callserviceimpl_c0ca292e3c8c0c97: (arg0: any, arg1: any, arg2: any, arg3: any, arg4: any, arg5: any, arg6: any) => {
                try {
                    let serviceId = getStringFromWasm0(wasm, arg1, arg2)
                    let fnName = getStringFromWasm0(wasm, arg3, arg4)
                    let args = getStringFromWasm0(wasm, arg5, arg6);
                    var ret = callService(serviceId, fnName, args);
                    let retStr = JSON.stringify(ret)
                    var ptr0 = passStringToWasm0(wasm, retStr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                    var len0 = WASM_VECTOR_LEN;
                    getInt32Memory0(wasm)[arg0 / 4 + 1] = len0;
                    getInt32Memory0(wasm)[arg0 / 4 + 0] = ptr0;
                } finally {
                    wasm.__wbindgen_free(arg1, arg2);
                    wasm.__wbindgen_free(arg3, arg4);
                    wasm.__wbindgen_free(arg5, arg6);
                }
            },
            __wbg_getcurrentpeeridimpl_a04b7c07e1108952: (arg0: any) => {
                var ret = pid.toB58String();
                var ptr0 = passStringToWasm0(wasm, ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
                var len0 = WASM_VECTOR_LEN;
                getInt32Memory0(wasm)[arg0 / 4 + 1] = len0;
                getInt32Memory0(wasm)[arg0 / 4 + 0] = ptr0;
            }
            },
        "host": { log_utf8_string: (arg0: any, arg1: any) => {
                try {
                    let str = getStringFromWasm0(wasm, arg0, arg1)
                    console.log(str)
                } finally {
                }
            }
        }
    };

    let module = await WebAssembly.compile(arr);
    let webAssemblyInstantiatedSource = await WebAssembly.instantiate(module, {
        ...importObject
    });

    wasm = webAssemblyInstantiatedSource.exports

    wasm.main();

    let func = (init_user_id: string, script: string, data: string) => {
        return aqua.invoke(wasm, init_user_id, script, data)
    }

    return func
}