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
    private client: Client;
    private tm: TendermintClient;
    private session: string;
    private sessionSummaryKey: string;
    private counter: number;
    private resultPromises: Set<ResultAwait>;
    private closed: boolean;
    private closedStatus: string;

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
    private targetKey(counter: number) {
        return `@meta/${this.client.id}/${this.session}/${counter}`;
    }

    /**
     * Cancels all promises that working now.
     */
    private cancelAllPromises(reason: string) {
        for (let resultAwait of this.resultPromises) {
            resultAwait.cancel(reason);
        }
        this.resultPromises = new Set<ResultAwait>();
    }

    /**
     * Closes session, cancel and delete all promises.
     */
    private closeSession(reason: string) {
        if (!this.closed) {
            this.closed = true;
            this.closedStatus = reason;
            this.cancelAllPromises(reason)
        }
    }

    private getCounterAndIncrement() {
        return this.counter++;
    }

    /**
     * Sends request with payload and wait for a response.
     *
     * @param payload a command supported by the program in a virtual machine with arguments
     */
    async invokeRaw(payload: string): Promise<Result> {

        // increments counter at the start, if some error occurred, other requests will be canceled in `cancelAllPromises`
        let currentCounter = this.getCounterAndIncrement();

        // throws an error immediately if the session is closed
        if (this.closed) {
            throw error(`Session is closed. Cause: ${this.closedStatus}`)
        }

        let txHex = genTxHex(this.client, this.session, currentCounter, payload);

        // send transaction
        let resp = await this.tm.broadcastTxSync(txHex);

        // close session if some error on sending transaction occurred
        if (resp.code !== 0) {
            let cause = `The session was closed after response with an error. Request payload: ${payload}, response: ${JSON.stringify(resp)}`;
            this.closeSession(cause);
            throw error(cause)
        }

        let targetKey = this.targetKey(currentCounter);

        // starts checking if a response has appeared
        let resultAwait = new ResultAwait(this.tm, targetKey, this.sessionSummaryKey);
        this.resultPromises = this.resultPromises.add(resultAwait);
        let pr: Promise<Result> = resultAwait.result();



        pr.then(() => {
                // delete promise from set if all ok
                this.resultPromises.delete(resultAwait);
            }
        ).catch((err: Error) => {
            // close session on error
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

        return this.invokeRaw(payload);
    }
}
