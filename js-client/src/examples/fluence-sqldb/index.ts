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

/**
 * The address of one node of a real-time cluster.
 */
interface Addr {
    host: string,
    port: number
}

/**
 * A status info of one node of a real-time cluster.
 */
interface Status {
    addr: string,
    block_hash: string,
    app_hash: string,
    block_height: number
}

interface PortWithClient {
    addr: string,
    client: fluence.TendermintClient
}

class DbClient {

    private readonly sessions: fluence.Session[];
    private readonly clients: PortWithClient[];
    private readonly size: number;
    private counter: number;

    private nodeNumber(): number {
        this.counter = (this.counter + 1) % this.size;
        return this.counter;
    }

    constructor(addrs: Addr[]) {
        this.size = addrs.length;
        this.counter = 0;

        this.sessions = addrs.map((v) => {
            // we can't use the same session with different nodes for now,
            // because we need to handle counter between different nodes in one session
            // it will be implemented in the client soon
            return fluence.createDefaultSession(v.host, v.port);
        });

        this.clients = addrs.map((v) => {
            return {
                addr: v.host + ":" + v.port,
                client: new fluence.TendermintClient(v.host, v.port)
            };
        });
    }

    /**
     * Submits queries to the real-time cluster and waits for a result.
     * @param queries list of queries to invoke
     */
    async submitQuery(queries: string[]): Promise<Promise<Result>[]> {
        let session = this.sessions[this.nodeNumber()];
        return queries.map((q) => {
            console.log("query: " + q);
            let res = session.invoke("do_query", q).result();
            res.then((r: Result) => {
                if (fluence.isValue(r)) {
                    let strResult = fluence.fromHex(r.hex());
                    console.log(`the result is:\n ${strResult}`);
                }
            });
            return res;
        });
    }

    /**
     * Gets status of all nodes.
     */
    async status(): Promise<any[]> {
        return Promise.all(this.clients.map((cl) => {
            let status = cl.client.client.status() as Promise<any>;
            return status.then((st) => {
                st.node_info.listen_addr = cl.addr;
                return st;
            });
        }));
    }
}

let btn = document.getElementById("submitQuery") as HTMLButtonElement;
let updateStatusBtn = document.getElementById("updateStatus") as HTMLButtonElement;
let resultField: HTMLTextAreaElement = window.document.getElementById("result") as HTMLTextAreaElement;
let inputField: HTMLInputElement = window.document.getElementById("query") as HTMLInputElement;
let statusField: HTMLTextAreaElement = window.document.getElementById("status") as HTMLTextAreaElement;

btn.addEventListener("click", () => {
    if (inputField.value.length !== 0) {
        submitQueries(inputField.value);
        inputField.value = "";
    }
});

//updates status of nodes every one second
let timer = setInterval(updateStatus, 1000);

//stops or starts the timer for status updates
updateStatusBtn.addEventListener("click", () => {
    let stop = "Stop update status";
    let start = "Start update status";
    if (updateStatusBtn.value === stop) {
        clearInterval(timer);
        updateStatusBtn.value = start;
    } else {
        timer = setInterval(updateStatus, 1000);
        updateStatusBtn.value = stop;
    }

});

function genStatus(status: Status) {
    return `<div class="m-2 rounded border list-group-item-info p-2">
                <label class="text-dark ml-2 mb-0" style="font-size: 0.8rem">${status.addr}</label>
                <ul class="list-unstyled mb-0 ml-4" style="font-size: 0.7rem">
                    <li>height: ${status.block_height}</li>
                    <li>block_hash: ${status.block_hash}</li>
                    <li>app_hash: ${status.app_hash}</li>
                </ul>
            </div>`
}

/**
 * Shortens string by getting only left and right part with given size.
 */
function shorten(str: string, size: number): string {
    return str.substring(0, size) + "..." + str.substring(str.length - size, str.length);
}

function updateStatus() {
    client.status().then((r) => {
        statusField.innerHTML = r.map((st) => {
            let info: any = st.sync_info;
            info.addr = st.node_info.listen_addr;

            let status: Status = {
                addr: st.node_info.listen_addr,
                block_hash: shorten(info.latest_block_hash, 10),
                app_hash: shorten(info.latest_app_hash, 10),
                block_height: info.latest_block_height
            };
            return genStatus(status)
        }).join("\n");
    })
}

interface Config {
    addrs: Addr[]
}

/**
 * List of addresses of a real-time cluster. Change it in `config.json` if needed.
 */
let config = require("./config.json") as Config;

console.log("Config: " + JSON.stringify(config));

let client = new DbClient(config.addrs);

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
