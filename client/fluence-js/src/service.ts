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

import {FluenceClient} from "./fluenceClient";

export class Service {
    private readonly client: FluenceClient;
    private readonly serviceId: string;
    private readonly peerId: string;

    constructor(client: FluenceClient, peerId: string, serviceId: string) {
        this.client = client;
        this.serviceId = serviceId;
        this.peerId = peerId;
    }

    /**
     *
     * @param moduleId wich module in service to call
     * @param args parameters to call service
     * @param fname function name if existed
     */
    async call(moduleId: string, args: any, fname?: string): Promise<any> {
        return this.client.callService(this.peerId, this.serviceId, moduleId, args, fname);
    }
}