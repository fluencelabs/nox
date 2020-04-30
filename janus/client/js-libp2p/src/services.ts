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

import {FunctionCall} from "./function_call";

export class Services {

    private services: Map<string, (req: FunctionCall) => void> = new Map();

    constructor() {}

    addService(serviceId: string, callback: (req: FunctionCall) => void, registerCall: FunctionCall): void {
        this.services.set(serviceId, callback);
    }

    getAllServices(): Map<string, (req: FunctionCall) => void> {
        return this.services;
    }

    deleteService(serviceId: string): boolean {
        return this.services.delete(serviceId)
    }

    // could throw error from service callback
    // returns true if the call was applied
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
