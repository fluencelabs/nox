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

import {RpcClient} from "tendermint"
import {none, Option, Some} from "ts-option";
import {fromHex} from "./utils";
import {BroadcastTxSyncResponse} from "./responses";

function parseResponse(res: any): BroadcastTxSyncResponse {
    let bResponse = <BroadcastTxSyncResponse> res;
    bResponse.data = fromHex(bResponse.data);
    return bResponse
}

export class TendermintClient {
    readonly client: RpcClient;

    constructor(host: string, port: number, protocol: protocol = "http") {
        this.client = new RpcClient(`${protocol}://${host}:${port}`);
    }

    /**
     * Sends broadcast_tx_sync operation.
     * @param hex transaction payload
     */
    broadcastTxSync(hex: string): Promise<BroadcastTxSyncResponse> {
        let params = {tx: JSON.stringify(hex)};
        return this.client.broadcastTxSync(params)
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
    async abciQuery(path: string): Promise<Option<any>> {
        let escaped = JSON.stringify(path);
        let response = (await this.client.abciQuery({path: escaped})).response;

        if (response.value) {
            let resultRaw = atob(response.value);

            let result = JSON.parse(resultRaw);
            return new Some(result);
        } else {
            return none;
        }

    }
}

//todo ws is not work for now due to some strange behavior with encoding
type protocol = "http" //| "ws"
