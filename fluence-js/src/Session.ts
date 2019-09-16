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

import {error, ErrorResponse, ErrorType, Result} from "./Result";
import {parseResponse, TendermintClient} from "./TendermintClient";
import {SessionConfig} from "./SessionConfig";

import Debug from "debug";
import {genSessionId, prepareRequest, PrivateKey, withSignature} from "./utils";
import {Option} from "ts-option";

const detailedDebug = Debug("request-detailed");
const txDebug = Debug("broadcast-request");

export enum RequestStatus {
    OK = 0,
    E_SESSION_CLOSED,
    E_REQUEST,
}

export interface RequestState<T> {
    status: RequestStatus;
    result?: T;
    error?: ErrorResponse;
}

/**
 * It is an identifier around which client can build a queue of requests.
 */
export class Session {
    readonly tm: TendermintClient;
    public readonly session: string;
    private readonly config: SessionConfig;
    private counter: number;
    private closed: boolean;
    private closedStatus: string;
    private readonly defaultBanTime: number;
    private lastBanTime: number;
    private bannedTill: number;

    /**
     * @param _tm transport to interact with the real-time cluster
     * @param _config parameters that regulate the session
     * @param _session session id, will be a random string with length 12 by default
     */
    constructor(_tm: TendermintClient, _config: SessionConfig,
                _session: string = genSessionId()) {
        this.tm = _tm;
        this.session = _session;
        this.config = _config;

        this.counter = 0;
        this.closed = false;
        this.defaultBanTime = 60000; // 60 sec by default
        this.lastBanTime = this.defaultBanTime;
        this.bannedTill = 0;
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

    banTime(): number {
        if (this.bannedTill === 0 || !this.isBanned()) return 0;
        else return this.bannedTill - Date.now();
    }

    private checkSession(): RequestState<any> | undefined {
        if (this.closed) {
            return {
                status: RequestStatus.E_SESSION_CLOSED,
                error: error(ErrorType.SessionClosed, `The session was closed. Cause: ${this.closedStatus}`)
            };
        }
    }

    async query(path: string): Promise<RequestState<Option<Result>>> {
        detailedDebug("start query");

        const sessionClosed = this.checkSession();
        if (sessionClosed) return sessionClosed;

        try {
            const queryResult = await this.tm.abciQuery(path);
            return {
                status: RequestStatus.OK,
                result: queryResult
            };
        } catch (err) {
            return {
                status: RequestStatus.E_REQUEST,
                error: error(ErrorType.TransportError, err.toString(), path),
            }
        }
    }

    /**
     * Sends request with payload, returns a response.
     *
     * @param payload Either an argument for Wasm VM main handler or a command for the statemachine
     * @param privateKey Optional private key to sign requests
     * @param counter Optional counter, overrides current counter
     */
    async request(payload: string, privateKey?: PrivateKey, counter?: number): Promise<RequestState<Option<Result>>> {

        detailedDebug("start request");

        const sessionClosed = this.checkSession();
        if (sessionClosed) return sessionClosed;

        const request = prepareRequest(payload, this.session, counter ? counter : this.getCounterAndIncrement(), privateKey);

        // send transaction
        txDebug("send broadcastTxSync");

        try {
            const txSendResult = await this.tm.txWaitResponse(request);
            return {
                status: RequestStatus.OK,
                result: txSendResult
            };
        } catch (err) {
            return {
                status: RequestStatus.E_REQUEST,
                error: error(ErrorType.TransportError, err.toString(), request.path),
            }
        }
    }

    /**
     * Sends request with payload, returns an id of response.
     *
     * @param payload Either an argument for Wasm VM main handler or a command for the statemachine
     * @param privateKey Optional private key to sign requests
     * @param counter Optional counter, overrides current counter
     */
    async requestAsync(payload: string, privateKey?: PrivateKey, counter?: number): Promise<RequestState<string>> {
        detailedDebug("start requestAsync");

        const sessionClosed = this.checkSession();
        if (sessionClosed) return sessionClosed;

        const request = prepareRequest(payload, this.session, counter ? counter : this.getCounterAndIncrement(), privateKey);

        // send transaction
        txDebug("send broadcastTxSync");
        let broadcastTxResultRaw;
        try {
            broadcastTxResultRaw = await this.tm.broadcastTxSync(request.payload);
        } catch (cause) {
            return {
                status: RequestStatus.E_REQUEST,
                error: error(ErrorType.TransportError, cause, request.path),
            }
        }

        let broadcastTxResult;
        try {
            broadcastTxResult = parseResponse(broadcastTxResultRaw);
        } catch (cause) {
            return {
                status: RequestStatus.E_REQUEST,
                error: error(ErrorType.ParsingError, cause, request.path),
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
                error: error(ErrorType.SessionClosed, cause, request.path),
            }
        }

        return {
            status: RequestStatus.OK,
            result: request.path,
        };
    }
}
