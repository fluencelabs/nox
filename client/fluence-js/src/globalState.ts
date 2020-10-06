import {Service} from "./call_service";
import {Particle} from "./particle";

// TODO put state with wasm file in each created FluenceClient
let services: Map<string, Service> = new Map();
let particlesQueue: Particle[] = [];
let currentParticle: string | undefined = undefined;

export function getCurrentParticleId(): string | undefined {
    return currentParticle;
}

export function setCurrentParticleId(particle: string | undefined) {
    currentParticle = particle;
}

export function addParticle(particle: Particle): void {
    particlesQueue.push(particle);
}

export function popParticle(): Particle | undefined {
    return particlesQueue.pop();
}

export function registerService(service: Service) {
    services.set(service.serviceId, service)
}

export function getService(serviceId: string): Service | undefined {
    return services.get(serviceId)
}
