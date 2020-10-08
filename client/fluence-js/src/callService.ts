import {getService, registerService} from "./globalState";

export interface CallServiceResult {
    ret_code: number,
    result: string
}

export class Service {

    serviceId: string;
    functions: Map<string, (args: any[]) => object> = new Map();

    constructor(serviceId: string) {
        this.serviceId = serviceId;
    }

    registerFunction(fnName: string, fn: (args: any[]) => object) {
        this.functions.set(fnName, fn);
    }

    call(fnName: string, args: any[]): CallServiceResult {
        let fn = this.functions.get(fnName)
        if (fn) {
            try {
                let result = fn(args)
                return {
                    ret_code: 0,
                    result: JSON.stringify(result)
                }
            } catch (err) {
                return {
                    ret_code: 1,
                    result: JSON.stringify(err)
                }
            }

        } else {
            let errorMsg = `Error. There is no function ${fnName}`
            return {
                ret_code: 1,
                result: JSON.stringify(errorMsg)
            }
        }
    }
}

let logger = new Service("")
logger.registerFunction("", (args: any[]) => {
    console.log("logger service: " + args)
    return { result: "done" }
})
registerService(logger);

export function callService(service_id: string, fn_name: string, args: string): CallServiceResult {
    try {
        let argsObject = JSON.parse(args)
        if (!Array.isArray(argsObject)) {
            throw new Error("args is not an array")
        }
        let service = getService(service_id)
        if (service) {
            return service.call(fn_name, argsObject)
        } else {
            return {
                result: JSON.stringify(`Error. There is no service: ${service_id}`),
                ret_code: 0
            }
        }
    } catch (err) {
        console.error("Cannot parse arguments: " + JSON.stringify(err))
    }

}
