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

export class Subscriptions {
    private subscriptions: Map<string,  (particle: Particle) => void> = new Map();

    constructor() {}

    /**
     * Subscriptions will be applied by outside message if id will be the same.
     *
     * @param id message identificator
     * @param f function to use with outside message
     * @param ttl time to live, subscription will be deleted after this time
     */
    subscribe(id: string, f: (particle: Particle) => void, ttl: number) {
        let _this = this;
        setTimeout(() => {
            _this.subscriptions.delete(id)
            console.log(`Particle with id ${id} deleted by timeout`)
        }, ttl)
        this.subscriptions.set(id, f);
    }

    /**
     * A particle will be applied if id of the particle was subscribed earlier.
     * @param particle
     */
    applyToSubscriptions(particle: Particle) {
        // if subscription return true - delete it from subscriptions
        let callback = this.subscriptions.get(particle.id)
        if (callback) {
            callback(particle);
        } else {
            if (particle.timestamp + particle.ttl > Date.now()) {
                console.log("Old particle received. 'ttl' is ended.");
            } else {
                console.log("External particle received. 'Stepper' needed on client. Unimplemented.");
            }
            console.log(particle);
        }
    }
}
