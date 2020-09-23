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
     * Subscriptions will be applied to all peer and relay messages.
     * If subscription returns true, delete subscription.
     * @param id
     * @param f
     */
    subscribe(id: string, f: (particle: Particle) => void) {
        this.subscriptions.set(id, f);
    }

    /**
     * Apply call to all subscriptions and delete subscriptions that return `true`.
     * @param particle
     */
    applyToSubscriptions(particle: Particle) {
        // if subscription return true - delete it from subscriptions
        let callback = this.subscriptions.get(particle.id)
        if (callback) {
            callback(particle);
        } else {
            console.log("External particle received. 'Stepper' needed on client. Unimplemented.");
            console.log(particle);
        }
    }
}
