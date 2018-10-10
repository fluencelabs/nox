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

import "bootstrap/dist/css/bootstrap.min.css";
import * as fluence from "js-fluence-client";
import {Result} from "js-fluence-client";

interface Addr {
    host: string,
    port: number
}

class DbClient {

    sessions: fluence.Session[];
    clients: fluence.TendermintClient[];
    size: number;
    counter: number;

    nodeNumber(): number {
        this.counter += 1;
        return this.counter % this.size;
    }

    constructor(addrs: Addr[]) {
        this.size = addrs.length;
        this.counter = 0;

        this.sessions = addrs.map((v) => {
            return fluence.createDefaultSession(v.host, v.port);
        });

        this.clients = addrs.map((v) => {
            return new fluence.TendermintClient(v.host, v.port);
        });
    }

    async submitQuery(queries: string[]): Promise<Promise<Result>[]> {
        return queries.map((q) => {
            console.log("query: " + q);
            let command = `do_query("${q}")`;
            let res = this.sessions[this.nodeNumber()].invokeRaw(command).result();
            res.then((r: Result) => {
                if (fluence.isValue(r)) {
                    let strResult = fluence.fromHex(r.hex());
                    console.log(`the result is:\n ${strResult}`);
                }
            });
            return res;
        });
    }

    async status(): Promise<any> {
        return this.clients[this.nodeNumber()].client.status();
    }
}

let btn = document.getElementById("submitQuery") as HTMLButtonElement;

btn.addEventListener("click", () => {
    if (inputField.value.length !== 0) {
        submitQueries(inputField.value);
        inputField.value = "";
    }

});

let statusBtn = document.getElementById("getStatus") as HTMLButtonElement;

statusBtn.addEventListener("click", () => {
    client.status().then((r) => {
        let info: any = r.sync_info;
        delete info.catching_up;
        info.addr = r.node_info.listen_addr;
        statusField.value = JSON.stringify(info, null, 2);
    })
});

let addrs = [
    {host: "localhost", port: 46157},
    {host: "localhost", port: 46257},
    {host: "localhost", port: 46357},
    {host: "localhost", port: 46457}
];
let client = new DbClient(addrs);

let resultField: HTMLTextAreaElement = window.document.getElementById("result") as HTMLTextAreaElement;
let inputField: HTMLInputElement = window.document.getElementById("query") as HTMLInputElement;
let statusField: HTMLTextAreaElement = window.document.getElementById("status") as HTMLTextAreaElement;

let newLine = String.fromCharCode(13, 10);
let sep = "**************************";

export function submitQueries(queries: string) {
    resultField.value = "";
    client.submitQuery(queries.split('\n')).then((results) => {

        results.forEach((pr) => {
            pr.then((r) => {
                if (fluence.isValue(r)) {
                    let strRes = r.asString().replace('\\n', newLine);
                    resultField.value += sep + newLine + strRes + newLine + sep;
                }
            });

        });

    })
}

const _global = (window /* browser */ || global /* node */) as any;
_global.client = client;
_global.DbClient = DbClient;