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
import * as fluence from "fluence";
import {AppSession, Result} from "fluence";
import {getNodeStatus} from "fluence-monitoring";

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

class DbClient {

    // sessions for every member of a cluster
    readonly appSession: AppSession;
    private readonly size: number;
    private counter: number;

    // round robin counter over app sessions
    private nextNodeIndex(): number {
        this.counter = (this.counter + 1) % this.size;
        return this.counter;
    }

    constructor(session: fluence.AppSession) {
        this.size = session.workerSessions.length;
        this.counter = 0;

        this.appSession = session;
    }

    /**
     * Submits queries to the real-time cluster and waits for a result.
     * @param queries list of queries to invoke
     */
    async submitQuery(queries: string[]): Promise<Promise<Result>[]> {
        let workerSession = this.appSession.workerSessions[this.nextNodeIndex()];
        return queries.map((q) => {
            console.log("query: " + q);
            let res = workerSession.session.invoke(q).result();
            res.then((r: Result) => {
                return r.asString()
            });
            return res;
        });
    }

    /**
     * Gets status of all nodes.
     */
    async status(): Promise<any[]> {
        return Promise.all(this.appSession.workerSessions.map((session) => {
            return getNodeStatus(session.node);
        }));
    }
}

let btn = document.getElementById("submitQuery") as HTMLButtonElement;
let updateStatusBtn = document.getElementById("updateStatus") as HTMLButtonElement;
let resultField: HTMLTextAreaElement = window.document.getElementById("result") as HTMLTextAreaElement;
let inputField: HTMLInputElement = window.document.getElementById("query") as HTMLInputElement;
let statusField: HTMLTextAreaElement = window.document.getElementById("status") as HTMLTextAreaElement;
let startBtn = document.getElementById("start") as HTMLButtonElement;
let main = document.getElementById("main") as HTMLDivElement;
let init = document.getElementById("init") as HTMLFormElement;
let metamaskCheckbox = document.getElementById("use-metamask") as HTMLInputElement;
let metamaskWrapper = document.getElementById("use-metamask-div") as HTMLDivElement;
let contractAddress: HTMLInputElement = window.document.getElementById("contract-address") as HTMLInputElement;
let appId: HTMLInputElement = window.document.getElementById("app-id") as HTMLInputElement;
let ethereumAddress: HTMLInputElement = window.document.getElementById("ethereum-address") as HTMLInputElement;

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

function genErrorStatus(addr: string, error: string) {
    return `<div class="m-2 rounded border list-group-item-info p-2">
                <label class="text-dark ml-2 mb-0" style="font-size: 0.8rem">${addr}</label>
                <ul class="list-unstyled mb-0 ml-4" style="font-size: 0.7rem">
                    <li>error: ${error}</li>
                </ul>
            </div>`
}

/**
 * Shortens string by getting only left and right part with given size.
 */
function shorten(str: string, size: number): string {
    return str.substring(0, size) + "..." + str.substring(str.length - size, str.length);
}

interface Config {
    addrs: Addr[]
}

let newLine = String.fromCharCode(13, 10);
let sep = "**************************";

async function preparePage(contractAddress: string, appId: string, ethereumAddress?: string) {
    init.hidden = true;
    main.hidden = false;

    let sessions = await fluence.connect(contractAddress, appId, ethereumAddress);

    let client = new DbClient(sessions);

    function updateStatus() {
        client.status().then((r) => {
            let addrs = client.appSession.workerSessions.map((s: any) => s.session.tm.addr);
            statusField.innerHTML = r.map((status, idx) => {
                let addr = addrs[idx];
                // if there is a response from a node
                if (status.workers) {
                    let workers: any[] = status.workers;
                    let worker = workers.find((w) => w.appId === parseInt(appId));
                    if (worker) {
                        if (!worker.isHealthy) {
                            return genErrorStatus(addr, "worker is not healthy")
                        } else if (worker.tendermint.http.HttpCheckStatus) {
                            let syncInfo = worker.tendermint.http.HttpCheckStatus.data.sync_info;
                            let status: Status = {
                                addr: addr,
                                block_hash: shorten(syncInfo.latest_block_hash as string, 10),
                                app_hash: shorten(syncInfo.latest_app_hash as string, 10),
                                block_height: syncInfo.latest_block_height as number
                            };
                            return genStatus(status)
                        } else if (worker.tendermint.http.HttpCheckFailed) {
                            return genErrorStatus(addr, `worker http check failed: ${worker.worker.http.HttpCheckFailed.cause}`)
                        } else {
                            return genErrorStatus(addr, "worker is not healthy")
                        }
                    } else {
                        return genErrorStatus(addr, "there is no worker with such appId")
                    }
                } else {
                    return genErrorStatus(addr, status.causeBy)
                }
            }).join("\n");
        })
    }

    // send request with query to a cluster
    function submitQueries(queries: string) {
        resultField.value = "";
        client.submitQuery(queries.split('\n')).then((results) => {
            results.forEach((pr) => {
                pr.then((r) => {
                    let strRes = r.asString().replace('\\n', newLine);
                    resultField.value += sep + newLine + strRes + newLine + sep;
                });

            });

        })
    }

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
}

startBtn.addEventListener("click", () => {
    if (contractAddress.value && appId.value && ethereumAddress.value) {
        let ethUrl = metamaskCheckbox.checked ? undefined : ethereumAddress.value;
        preparePage(contractAddress.value, appId.value, ethUrl);
    } else {
        contractAddress.reportValidity();
        appId.reportValidity();
        ethereumAddress.reportValidity();
    }
});

window.addEventListener('load', function() {

    let web3 = (window as any).web3;

    // Check if Web3 has been injected by the browser (Mist/MetaMask).
    if (typeof web3 === 'undefined') {
        metamaskWrapper.hidden = true;
    } else {
        metamaskCheckbox.addEventListener( 'change', function() {
            ethereumAddress.disabled = this.checked;
        });
    }
});
