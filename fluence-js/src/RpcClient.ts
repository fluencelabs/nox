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

import axios, {AxiosPromise} from 'axios';

// Client to interaction with tendermint client through master node proxy
export class RpcClient {

    url: string;

    constructor(addr: string, appId: string) {
        this.url = `${addr}/apps/${appId}`;
    }

    broadcastTxSync(tx: string): AxiosPromise<any> {
        let url = `${this.url}/tx`;
        return axios.post(url, tx)
    }

    abciQuery(path: string): AxiosPromise<any> {
        return axios.get(`${this.url}/query`, {
            params: {
                path: path,
                data: ""
            }
        })
    }
}
