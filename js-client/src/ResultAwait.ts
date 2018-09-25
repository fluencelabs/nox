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

import {empty, error, Result, timeout, value} from "./Result";
import {TendermintClient} from "./TendermintClient";

/**
 * Class with the ability to make request periodically until an answer is available.
 */
export class ResultAwait {
    private tm: TendermintClient;
    private target_key: string;

    constructor(_tm: TendermintClient, _target_key: string) {
        this.tm = _tm;
        this.target_key = _target_key;
    }

    /**
     * Creates promise that will wait `ms` milliseconds.
     *
     * @param ms milliseconds to wait
     */
    async sleep(ms: number) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * Periodically checks the node of the real-time cluster for the presence of a result.
     *
     * @param requestsPerSec check frequency
     * @param responseTimeoutSec what time to check
     */
    async result(requestsPerSec: number = 4, responseTimeoutSec = 5): Promise<Result> {
        const path = JSON.stringify(this.target_key + "/result");

        for (let _i = 0; _i < responseTimeoutSec * requestsPerSec; _i++) {
            const statusResponse = (await this.tm.client.abciQuery({path: path})).response;

            if (statusResponse.value) {

                const result = atob(statusResponse.value);

                const resultJs = JSON.parse(result);

                if (resultJs.Error !== undefined) {
                    return error(resultJs.Error.message)
                } else if (resultJs.Empty !== undefined) {
                    return empty;
                } else {
                    return value(resultJs.Computed.value);
                }
            }

            await this.sleep(1000 / requestsPerSec)

        }
        return timeout;
    }
}
