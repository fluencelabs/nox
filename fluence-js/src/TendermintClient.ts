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

import {none, Option, Some} from "ts-option";
import {fromHex} from "./utils";
import * as debug from "debug";
import {RpcClient} from "./RpcClient";
import {error, Result, value} from "./Result";

const d = debug("tendermintClient");

interface BroadcastTxSyncResponse {
    code: number
    data: string
    log: string
    hash: string
}

function parseResponse(res: any): BroadcastTxSyncResponse {
     try {
         let bResponse = <BroadcastTxSyncResponse> res.data.result;
         bResponse.data = fromHex(bResponse.data);
         return bResponse;
     } catch (e) {
         throw new Error(`Cannot parse the response because of an error: ${JSON.stringify(e)}\n Response: ${JSON.stringify(res)}`);
     }
}

export class TendermintClient {
    readonly client: RpcClient;
    readonly addr: string;
    readonly appId: string;

    constructor(host: string, port: number, appId: string, protocol: protocol = "http") {
        this.addr = `${protocol}://${host}:${port}`;
        this.appId = appId;
        this.client = new RpcClient(this.addr, appId);
    }

    /**
     * Sends broadcast_tx_sync operation.
     * @param payload transaction payload
     */
    broadcastTxSync(payload: string): Promise<BroadcastTxSyncResponse> {
        d("broadCastTxSync request");
        return this.client.broadcastTxSync(payload)
            .then((res: any) => {
                return parseResponse(res);
            }).catch((err: any) => {
                return parseResponse(err);
            });
    }

    /**
     * Sends an ABCI query.
     * @param path query parameter
     *
     * @returns `none` if there is no value, and `some` with parsed from hex value otherwise.
     */
    async abciQuery(path: string): Promise<Option<Result>> {
        d("abciQuery request");

        let response: any = (await this.client.abciQuery(path)).data.result.response;

        function getResult(): any {
            try {
                return atob(response.value);
            } catch (e) {
                throw error("error on atob(response.value): " + JSON.stringify(response) + " err: " + e);
            }
        }

        if (response.value) {
            if (response.code === undefined || response.code === 0) {
                // code == OK
                return new Some(value(getResult()))
            } else if (response.code === 1 || response.code === 2) {
                // code == CannotParseHeader || Dropped
                let result = getResult();
                throw error("bad code on response: " + JSON.stringify(response) + " result: " + JSON.stringify(result));
            } else if (response.code === 3 || response.code === 4) {
                // code == Pending || NotFound
                return none;
            } else {
                let result = getResult();
                throw error("unknown code on response: " + JSON.stringify(response) + " result: " + JSON.stringify(result));
            }
        } else {
            return none;
        }
    }
}

//todo ws is not work for now due to some strange behavior with encoding
type protocol = "http" //| "ws"
