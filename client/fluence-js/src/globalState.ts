import {Service} from "./call_service";

// TODO put state with wasm file in each created FluenceClient
let services: Map<string, Service> = new Map();

export function registerService(service: Service) {
    services.set(service.serviceId, service)
}

export function getService(serviceId: string): Service | undefined {
    return services.get(serviceId)
}