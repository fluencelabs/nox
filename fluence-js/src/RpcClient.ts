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

import axios, {AxiosPromise, AxiosRequestConfig} from 'axios';
import {QueryResponse} from './Result';
import {BroadcastTxSyncResponse} from './TendermintClient';

interface TendermintJsonRpcResponse<T = any> {
    id: any;
    jsonrpc: string;
    result: T;
}

interface AbciQueryResult {
    response: QueryResponse;
}

// Client to interaction with tendermint client through master node proxy
export class RpcClient {

    url: string;
    private config: AxiosRequestConfig;

    constructor(addr: string, appId: string) {
        this.url = `${addr}/apps/${appId}`;
        this.config = {
            timeout: 5000, // 5 sec timeout by default
        };
    }

    broadcastTxSync(tx: string): AxiosPromise<TendermintJsonRpcResponse<BroadcastTxSyncResponse>> {
        let url = `${this.url}/tx`;
        return axios.post(url, tx, this.config)
    }

    abciQuery(path: string): AxiosPromise<TendermintJsonRpcResponse<AbciQueryResult>> {
        return axios.get(`${this.url}/query`, {
            ...this.config,
            params: {
                path: path,
                data: ""
            }
        })
    }
}
