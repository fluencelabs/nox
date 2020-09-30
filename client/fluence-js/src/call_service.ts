class Service {

    serviceId: string;
    functions: Map<string, (args: string) => string> = new Map();

    constructor(serviceId: string) {
        this.serviceId = serviceId;
    }

    registerFunction(fnName: string, fn: (args: string) => string) {
        this.functions.set(fnName, fn);
    }

    call(fnName: string, args: string): string {
        let fn = this.functions.get(fnName)
        if (fn) {
            return fn(args)
        } else {
            return `Error. There is no function ${fnName}`
        }
    }
}

let services: Map<string, Service> = new Map();

let logger = new Service("logger")
logger.registerFunction("logger", (args: string) => {
    console.log("logger service: " + args)
    return ""
})
services.set("logger", logger)

export function call_service(service_id: string, fn_name: string, args: string): string {
    let service = services.get(service_id)
    if (service) {
        return service.call(fn_name, args)
    } else {
        return `Error. There is no service ${service_id}`
    }
}