/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

import {Service} from "./callService";
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

export function deleteService(serviceId: string): boolean {
    return services.delete(serviceId)
}

export function getService(serviceId: string): Service | undefined {
    return services.get(serviceId)
}
