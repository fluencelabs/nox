/*
 * Copyright 2018 Fluence Labs Limited
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

import {ResultAwait} from "./ResultAwait";
import {Result} from "./Result";
import {genTxHex} from "./tx";
import {TendermintClient} from "./TendermintClient";
import {Client} from "./Client";

/**
 * It is an identifier around which client can build a queue of requests.
 */
export class Session {
    client: Client;
    tm: TendermintClient;
    session: string;
    statusKey: string;
    counter: number;

    private static genSessionId() {
        let randomstring = require("randomstring");
        return randomstring.generate(12);
    }

    /**
     * @param _tm transport to interact with the real-time cluster
     * @param _client an identifier and a signer
     * @param _session session id, will be a random string with length 12 by default
     */
    constructor(_tm: TendermintClient, _client: Client, _session: string = Session.genSessionId()) {
        this.tm = _tm;
        this.client = _client;

        this.session = _session;
        this.counter = 0;

        this.statusKey = `@meta/${this.client.id}/${this.session}/@session_status`;
    }

    /**
     * Generates a key, that will be an identifier of the request.
     */
    private targetKey() {
        return `@meta/${this.client.id}/${this.session}/${this.counter}`;
    }

    /**
     * Sends request with payload and wait for a response.
     *
     * @param payload a command supported by the program in a virtual machine with arguments
     */
    async submitRaw(payload: string): Promise<Result> {

        let txHex = genTxHex(this.client, this.session, this.counter, payload);

        let params = {tx: txHex};

        //todo check if this request is successful
        await this.tm.client.broadcastTxSync(params);

        let targetKey = this.targetKey();

        //todo there should be a manager that syncs calls and increments counter only after a successful request
        this.counter = this.counter + 1;

        return new ResultAwait(this.tm, targetKey).result()
    }

    /**
     * Sends request with a payload and wait for a response.
     *
     * @param command a command supported by the program in a virtual machine
     * @param args arguments for command
     */
    async submit(command: string, args: string[] = []): Promise<Result> {

        let payload = command + `(${args.join(',')})`;

        return this.submitRaw(payload);
    }
}
