import {FunctionCall} from "./function_call";

export class Services {

    private services: Map<string, (req: FunctionCall) => void> = new Map();

    constructor() {}

    addService(serviceId: string, callback: (req: FunctionCall) => void): void {
        this.services.set(serviceId, callback)
    }

    deleteService(serviceId: string): boolean {
        return this.services.delete(serviceId)
    }

    // could throw error from service callback
    applyToService(serviceId: string, call: FunctionCall): boolean {
        let service = this.services.get(serviceId);
        if (service) {
            service(call);
            return true;
        } else {
            return false;
        }

    }
}
