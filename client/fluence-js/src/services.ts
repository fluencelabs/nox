/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

import {FunctionCall} from "./function_call";

export class Services {

    private services: Map<string, (req: FunctionCall) => void> = new Map();

    constructor() {}

    addService(serviceId: string, callback: (req: FunctionCall) => void): void {
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
