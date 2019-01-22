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

import {ResultAwait, ResultError, ResultPromise} from "./ResultAwait";
import {error, ErrorResult, Result} from "./Result";
import {genTxHex} from "./tx";
import {TendermintClient} from "./TendermintClient";
import {Client} from "./Client";
import {SessionConfig} from "./SessionConfig";

import  * as debug from "debug";
import {toHex} from "./utils";

const detailedDebug = debug("invoke-detailed");
const txDebug = debug("broadcast-request");

/**
 * It is an identifier around which client can build a queue of requests.
 */
export class Session {
    private readonly client: Client;
    private readonly tm: TendermintClient;
    private readonly session: string;
    private readonly sessionSummaryKey: string;
    private readonly config: SessionConfig;
    private counter: number;
    private lastResult: ResultAwait;
    private closing: boolean;
    private closed: boolean;
    private closedStatus: string;

    private static genSessionId() {
        let randomstring = require("randomstring");
        return randomstring.generate(12);
    }

    /**
     * @param _tm transport to interact with the real-time cluster
     * @param _client an identifier and a signer
     * @param _config parameters that regulate the session
     * @param _session session id, will be a random string with length 12 by default
     */
    constructor(_tm: TendermintClient, _client: Client, _config: SessionConfig,
                _session: string = Session.genSessionId()) {
        this.tm = _tm;
        this.client = _client;
        this.session = _session;
        this.config = _config;

        this.counter = 0;
        this.closed = false;
        this.closing = false;

        this.sessionSummaryKey = `@meta/${this.client.id}/${this.session}/@sessionSummary`;
    }

    /**
     * Generates a key, that will be an identifier of the request.
     */
    private targetKey(counter: number) {
        return `@meta/${this.client.id}/${this.session}/${counter}`;
    }

    /**
     * Marks session as closed.
     */
    private markSessionAsClosed(reason: string) {
        if (!this.closed) {
            this.closed = true;
            this.closedStatus = reason;
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
    invokeRaw(payload: string): ResultPromise {
        // throws an error immediately if the session is closed
        if (this.closed) {
            return new ResultError(`The session was closed. Cause: ${this.closedStatus}`)
        }

        if (this.closing) {
            this.markSessionAsClosed(this.closedStatus)
        }

        detailedDebug("start invoke");

        // increments counter at the start, if some error occurred, other requests will be canceled in `cancelAllPromises`
        let currentCounter = this.getCounterAndIncrement();

        let txHex = genTxHex(this.client, this.session, currentCounter, payload);

        // send transaction
        txDebug("send broadcastTxSync");
        let broadcastRequestPromise: Promise<void> = this.tm.broadcastTxSync(txHex).then((resp: any) => {
            detailedDebug("broadCastTxSync response received");
            txDebug("broadCastTxSync response received");
            // close session if some error on sending transaction occurred
            if (resp.code !== 0) {
                let cause = `The session was closed after response with an error. Request payload: ${payload}, response: ${JSON.stringify(resp)}`;
                this.markSessionAsClosed(cause);
                throw error(cause)
            }
        });

        let targetKey = this.targetKey(currentCounter);

        let callback = (err: ErrorResult) => {
            // close session on error
            this.markSessionAsClosed(err.error)
        };

        let resultAwait = new ResultAwait(this.tm, this.config, targetKey, this.sessionSummaryKey,
            broadcastRequestPromise, callback);
        this.lastResult = resultAwait;

        return resultAwait;
    }

    /**
     * Sends request with a payload and wait for a response.
     *
     * @param arg argument for command
     * @param moduleName name of module that should be called
     */
    invoke(arg: string = "", moduleName: string = ""): ResultPromise {

        let payload: string = moduleName + `(${toHex(arg)})`;

        return this.invokeRaw(payload);
    }

    /**
     * Syncs on all pending invokes.
     */
    async sync(): Promise<Result> {
        return this.lastResult.result();
    }

    /**
     * Closes session locally and send a command to close the session on the cluster.
     */
    close(reason: string = ""): ResultPromise {
        this.closing = true;
        this.closedStatus = reason;
        let result = this.invoke("@closeSession");
        result.result();
        return result;
    }
}
