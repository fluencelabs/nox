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

import {empty, error, Result, value} from "./Result";
import {TendermintClient} from "./TendermintClient";
import {none, Option} from "ts-option";
import {SessionSummary} from "./responses";

/**
 * Class with the ability to make request periodically until an answer is available.
 */
export class ResultAwait {
    private readonly tm: TendermintClient;
    private readonly targetKey: string;
    private readonly summaryKey: string;
    private canceled: boolean;
    private canceledReason: string;
    private invokeResult: Promise<Result>;

    /**
     *
     * @param _tm transport to the real-time cluster
     * @param _targetKey key to check restul from cluster
     * @param _summaryKey key to check session info from cluster
     */
    constructor(_tm: TendermintClient, _targetKey: string, _summaryKey: string) {
        this.tm = _tm;
        this.targetKey = _targetKey;
        this.summaryKey = _summaryKey;
        this.canceled = false;
    }

    private async getSessionInfo(): Promise<Option<SessionSummary>> {
        const sessionInfo: Option<any> = (await this.tm.abciQuery(this.summaryKey));
        return sessionInfo.map((info: any) => {
            return <SessionSummary> info
        });
    }

    /**
     * Creates promise that will wait `ms` milliseconds.
     *
     * @param ms milliseconds to wait
     */
    private async sleep(ms: number) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * Periodically checks the node of the real-time cluster for the presence of a result.
     * If the result is already obtained, return it without new calculations.
     *
     * @param requestsPerSec check frequency
     * @param responseTimeoutSec what time to check
     * @param requestTimeout the time after which the error occurs if the result has not yet been received
     */
    async result(requestsPerSec: number = 2, responseTimeoutSec: number = 10, requestTimeout: number = 60): Promise<Result> {

        if (this.invokeResult === undefined) {
            const path = this.targetKey + "/result";

            let pr = this.checkResultPeriodically(path, requestsPerSec, responseTimeoutSec, requestTimeout);

            this.invokeResult = pr;

            return pr;
        } else {
            return this.invokeResult
        }



    }

    /**
     * Sends request for a result and parse it.
     * @param path
     * @returns `none` if there is no result, `some` if result appeared and throws an error if result is an error
     */
    private async checkResult(path: string): Promise<Option<Result>> {

        const statusResponse: Option<any> = (await this.tm.abciQuery(path));

        return statusResponse.map((res: any) => {
            if (res.Error !== undefined) {
                throw error(res.Error.message)
            } else if (res.Empty !== undefined) {
                return empty;
            } else {
                return value(res.Computed.value);
            }
        });
    }

    /**
     * Checks the result until it appears or until an error occurs or the session is closed.
     * @param path address to check result
     * @param requestsPerSec the frequency of requests to check per second
     * @param responseTimeoutSec the time after which it will check the session for activity too
     * @param requestTimeout the time after which the error occurs if the result has not yet been received
     * @returns result or error if some error occurred or session is closed
     */
    private async checkResultPeriodically(path: string, requestsPerSec: number, responseTimeoutSec: number,
                                          requestTimeout: number): Promise<Result> {

        let _i: number = 0;
        let sessionInfo: Option<SessionSummary> = none;

        while(true) {

            // checking result was canceled outside
            if (this.canceled) {
                throw error(`The request was canceled. Cause: ${this.canceledReason}`)
            }

            if (_i > requestsPerSec * requestTimeout) {
                throw error(`The request was timouted after ${requestTimeout} seconds.`)
            }

            // checks the session after some tries
            let checkSession = _i > responseTimeoutSec * requestsPerSec;

            if (checkSession) {
                sessionInfo = await this.getSessionInfo();
            }

            let optionResult = await this.checkResult(path);

            // if result exists, return it
            if (optionResult.nonEmpty) {
                return optionResult.get
            }

            _i++;

            // here the session is checked after the result is checked, as it may happen that the result is given,
            // and after that the session was immediately closed
            if (sessionInfo.nonEmpty) {
                if (sessionInfo.get.status.Active === undefined) {
                    throw error(`Session is ${JSON.stringify(sessionInfo.get.status)}`)
                }
            }

            // wait for next check
            await this.sleep(1000 / requestsPerSec);
        }
    }

    /**
     * Cancels result checking.
     */
    cancel(reason: string) {
        this.canceled = true;
        this.canceledReason = reason;
    }
}
