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
import {error, ErrorResponse, Result} from "./Result";
import {TendermintClient} from "./TendermintClient";
import {SessionConfig} from "./SessionConfig";

import * as debug from "debug";
import {PrivateKey, withSignature} from "./utils";
import * as randomstring from "randomstring";

const detailedDebug = debug("request-detailed");
const txDebug = debug("broadcast-request");

export enum RequestStatus {
    OK = 0,
    E_SESSION_CLOSED,
    E_REQUEST,
}

interface RequestState {
    status: RequestStatus;
    result?: ResultPromise;
    error?: ErrorResponse;
}

/**
 * It is an identifier around which client can build a queue of requests.
 */
export class Session {
    readonly tm: TendermintClient;
    private readonly session: string;
    private readonly config: SessionConfig;
    private counter: number;
    private lastResult: ResultAwait;
    private closing: boolean;
    private closed: boolean;
    private closedStatus: string;
    private readonly defaultBanTime: number;
    private lastBanTime: number;
    private bannedTill: number;

    static genSessionId(): string {
        return randomstring.generate(12);
    }

    /**
     * @param _tm transport to interact with the real-time cluster
     * @param _config parameters that regulate the session
     * @param _session session id, will be a random string with length 12 by default
     */
    constructor(_tm: TendermintClient, _config: SessionConfig,
                _session: string = Session.genSessionId()) {
        this.tm = _tm;
        this.session = _session;
        this.config = _config;

        this.counter = 0;
        this.closed = false;
        this.closing = false;
        this.defaultBanTime = 60000; // 60 sec by default
        this.lastBanTime = this.defaultBanTime;
        this.bannedTill = 0;
    }

    /**
     * Generates a key, that will be an identifier of the request.
     */
    private targetKey(counter: number) {
        return `${this.session}/${counter}`;
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

    /**
     * Increments current internal counter
     */
    private getCounterAndIncrement() {
        return this.counter++;
    }

    /**
     * Ban session for usage for some time
     *
     * @param banTimeMs Milliseconds to ban for
     */
    ban(banTimeMs?: number): void {
        if (banTimeMs) {
            this.lastBanTime = banTimeMs;
            this.bannedTill = Date.now() + banTimeMs;
        } else {
            if (this.bannedTill > Date.now()) {
                return;
            }

            if (Date.now() < this.bannedTill + this.lastBanTime) {
                this.lastBanTime *= 2;
            } else {
                this.lastBanTime = this.defaultBanTime;
            }

            this.bannedTill = Date.now() + this.lastBanTime;
        }
    }

    /**
     * Checks if session banned for usage
     */
    isBanned(): boolean {
        return this.bannedTill >= Date.now();
    }

    /**
     * Sends request with payload and wait for a response.
     *
     * @param payload Either an argument for Wasm VM main handler or a command for the statemachine
     * @param privateKey Optional private key to sign requests
     * @param counter Optional counter, overrides current counter
     */
    async request(payload: string, privateKey?: PrivateKey, counter?: number): Promise<RequestState> {
        // throws an error immediately if the session is closed
        if (this.closed) {
            return {
                status: RequestStatus.E_SESSION_CLOSED,
                error: error(`The session was closed. Cause: ${this.closedStatus}`)
            };
        }

        if (this.closing) {
            this.markSessionAsClosed(this.closedStatus)
        }

        detailedDebug("start request");

        // increments counter at the start, if some error occurred, other requests will be canceled in `cancelAllPromises`
        let currentCounter = counter ? counter : this.getCounterAndIncrement();

        let signed = withSignature(payload, currentCounter, privateKey);
        let tx = `${this.session}/${currentCounter}\n${signed}`;

        // send transaction
        txDebug("send broadcastTxSync");
        let broadcastTxResult;
        try {
            broadcastTxResult = await this.tm.broadcastTxSync(tx);
        } catch (err) {
            return {
                status: RequestStatus.E_REQUEST,
                error: error(`Request error on broadcastTx occured. Request payload: ${payload}, error: ${JSON.stringify(err)}`),
            }
        }

        detailedDebug("broadCastTxSync response received");
        txDebug("broadCastTxSync response received");

        // close session if some error on sending transaction occurred
        if (broadcastTxResult.code !== 0) {
            const cause = `The session was closed after response with an error. Request payload: ${payload}, response: ${JSON.stringify(broadcastTxResult)}`;
            this.markSessionAsClosed(cause);
            return {
                status: RequestStatus.E_SESSION_CLOSED,
                error: error(cause),
            }
        }

        const targetKey = this.targetKey(currentCounter);

        const callback = (err: ErrorResponse) => {
            // close session on error
            this.markSessionAsClosed(err.error)
        };

        const resultAwait = new ResultAwait(this.tm, this.config, targetKey, this.session, callback);
        this.lastResult = resultAwait;

        return {
            status: RequestStatus.OK,
            result: resultAwait,
        };
    }

    /**
     * Syncs on all pending requests.
     */
    async sync(): Promise<Result> {
        return this.lastResult.result();
    }
}
