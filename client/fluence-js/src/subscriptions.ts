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

import {Particle} from "./particle";
import log from "loglevel";

export class Subscriptions {
    private subscriptions: Map<string, Particle> = new Map();

    constructor() {}

    /**
     * Subscriptions will be applied by outside message if id will be the same.
     *
     * @param particle
     * @param ttl time to live, subscription will be deleted after this time
     */
    subscribe(particle: Particle, ttl: number) {
        let _this = this;
        setTimeout(() => {
            _this.subscriptions.delete(particle.id)
            log.info(`Particle with id ${particle.id} deleted by timeout`)
        }, ttl)
        this.subscriptions.set(particle.id, particle);
    }

    update(particle: Particle): boolean {
        if (this.subscriptions.has(particle.id)) {
            this.subscriptions.set(particle.id, particle);
            return true;
        } else {
            return false;
        }
    }

    get(id: string): Particle | undefined {
        return this.subscriptions.get(id)
    }

    hasSubscription(particle: Particle): boolean {
        return this.subscriptions.has(particle.id)
    }
}
