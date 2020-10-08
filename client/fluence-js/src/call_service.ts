import {getService, registerService} from "./globalState";

interface CallServiceResult {
    ret_code: number,
    result: string
}

export class Service {

    serviceId: string;
    functions: Map<string, (args: string) => object> = new Map();

    constructor(serviceId: string) {
        this.serviceId = serviceId;
    }

    registerFunction(fnName: string, fn: (args: string) => object) {
        this.functions.set(fnName, fn);
    }

    call(fnName: string, args: string): CallServiceResult {
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
logger.registerFunction("", (args: string) => {
    console.log("logger service: " + args)
    return { result: "done" }
})
registerService(logger);

export function call_service(service_id: string, fn_name: string, args: string): CallServiceResult {
    let service = getService(service_id)
    if (service) {
        return service.call(fn_name, args)
    } else {
        return {
            result: JSON.stringify(`Error. There is no service: ${service_id}`),
            ret_code: 0
        }
    }
}
