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
import {Error, error, Result} from "./Result";
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
    sessionSummaryKey: string;
    counter: number;
    resultPromises: Set<ResultAwait>;
    closed: boolean;
    closedStatus: string;

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

        this.resultPromises = new Set<ResultAwait>();

        this.sessionSummaryKey = `@meta/${this.client.id}/${this.session}/@sessionSummary`;

        this.closed = false;
    }

    /**
     * Generates a key, that will be an identifier of the request.
     */
    private targetKey() {
        return `@meta/${this.client.id}/${this.session}/${this.counter}`;
    }

    private cancelAndClearPromises(reason: string) {
        for (let resultAwait of this.resultPromises) {
            resultAwait.cancel(reason);
        }
        this.resultPromises = new Set<ResultAwait>();
    }

    private closeSession(reason: string) {
        if (!this.closed) {
            this.closed = true;
            this.closedStatus = reason;
            this.cancelAndClearPromises(reason)
        }
    }

    /**
     * Sends request with payload and wait for a response.
     *
     * @param payload a command supported by the program in a virtual machine with arguments
     */
    async submitRaw(payload: string): Promise<Result> {

        if (this.closed) {
            throw error(`Session is closed. Cause: ${this.closedStatus}`)
        }

        let txHex = genTxHex(this.client, this.session, this.counter, payload);

        //todo check if this request is successful
        let resp = await this.tm.broadcastTxSync(txHex);

        if (resp.code !== 0) {
            let cause = `The session was closed after response with an error. Request payload: ${payload}, response: ${JSON.stringify(resp)}`
            this.closeSession(cause);
            throw error(cause)
        }

        let targetKey = this.targetKey();

        //todo there should be a manager that syncs calls and increments counter only after a successful request
        this.counter = this.counter + 1;

        let resultAwait = new ResultAwait(this.tm, targetKey, this.sessionSummaryKey);
        let pr: Promise<Result> = resultAwait.result();

        this.resultPromises = this.resultPromises.add(resultAwait);

        pr.then((res: Result) => {
                this.resultPromises.delete(resultAwait);
            }
        ).catch((err: Error) => {
            //check if session already closed, if not - close it, drop all promises
            this.closeSession(err.error)
        });

        return pr;
    }

    /**
     * Sends request with a payload and wait for a response.
     *
     * @param command a command supported by the program in a virtual machine
     * @param args arguments for command
     */
    async invoke(command: string, args: string[] = []): Promise<Result> {

        let payload = command + `(${args.join(',')})`;

        return this.submitRaw(payload);
    }
}
