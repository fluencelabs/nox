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

import {FunctionCall} from "./functionCall";
import {Address} from "./address";

export class Subscriptions {
    private subscriptions: ((args: any, target: Address, replyTo: Address, module?: string, fname?: string) => (boolean | undefined))[] = [];

    constructor() {}

    /**
     * Subscriptions will be applied to all peer and relay messages.
     * If subscription returns true, delete subscription.
     * @param f
     */
    subscribe(f: (args: any, target: Address, replyTo: Address, moduleId?: string, fname?: string) => (boolean | undefined)) {
        this.subscriptions.push(f);
    }

    /**
     * Apply call to all subscriptions and delete subscriptions that return `true`.
     * @param call
     */
    applyToSubscriptions(call: FunctionCall) {
        // if subscription return false - delete it from subscriptions
        this.subscriptions = this.subscriptions.filter(callback => !callback(call.arguments, call.target, call.reply_to, call.module, call.fname))
    }
}
