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

import {genRequestId, genSessionId, prepareRequest, PrivateKey} from "./utils";
import {Node} from "./contract";
import {toByteArray} from "base64-js";
import {Result} from "./fluence";

enum RequestType {
    Query,
    RequestAsync
}

interface Request {
    payload: string,
    requestType: RequestType,
    header?: string
}

interface Subscription {
    subscriptionId: string,
    tx: string
}

interface WebsocketResponse {
    request_id: string
    type: string
    data?: string
    error?: string
}

interface ResultExecutor {
    resultHandler: (result: Result) => void
    errorHandler: (error: any) => void
    subscription: boolean
}

export class WebsocketSession {
    private sessionId: string;
    private appId: string;
    private readonly privateKey?: PrivateKey;
    private counter: number;
    private nodes: Node[];
    private isOpen: boolean;
    private nodeCounter: number;
    private socket: WebSocket;

    private waitingRequests = new Map<string, ResultExecutor>();

    constructor(appId: string, nodes: Node[], privateKey?: PrivateKey) {
        if (nodes.length == 0) {
            console.error("There is no nodes to connect");
            throw new Error("There is no nodes to connect");
        }

        this.counter = 0;
        this.nodeCounter = 0;
        this.sessionId = genSessionId();
        this.appId = appId;
        this.nodes = nodes;
        this.privateKey = privateKey;

        this.connect(appId, nodes[this.nodeCounter + 1]);
    }

    /**
     * Increments current internal counter
     */
    private getCounterAndIncrement() {
        return this.counter++;
    }

    subscribe(payload: string, resultHandler: (result: Result) => void, errorHandler: (error: any) => void): void {
        let requestId = genRequestId();
        let subscriptionId = genRequestId();

        let request = {
            tx: payload,
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "subscribe_request"
        };

        this.socket.send(JSON.stringify(request));

        let executor: ResultExecutor = {
            resultHandler: resultHandler,
            errorHandler: errorHandler,
            subscription: true
        };

        this.waitingRequests.set(subscriptionId, executor);
    }

    requestAsync(payload: string): void {
        let requestId = genRequestId();
        let counter = this.getCounterAndIncrement();

        let tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        let request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx"
        };

        console.log("send requestAsync: " + JSON.stringify(request));

        this.socket.send(JSON.stringify(request));
    }

    request(payload: string, resultHandler: (result: Result) => void, errorHandler: (error: any) => void): void {
        let requestId = genRequestId();
        let counter = this.getCounterAndIncrement();

        let tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        let request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_wait_request"
        };

        console.log("send request: " + JSON.stringify(request));

        this.socket.send(JSON.stringify(request));

        let executor: ResultExecutor = {
            resultHandler: resultHandler,
            errorHandler: errorHandler,
            subscription: false
        };

        this.waitingRequests.set(requestId, executor);
    }

    private resetSession() {
        this.sessionId = genSessionId();
        this.counter = 0;
    }

    private static parseRawResponse(response: string): WebsocketResponse {
        let parsed = JSON.parse(response);
        if (!parsed.request_id) throw new Error("Cannot parse response, no 'request_id' field.");
        if (parsed.type === "tx_wait_response" && !parsed.data && !parsed.error) throw new Error(`Cannot parse response, no 'data' or 'error' field in response with requestId '${parsed.requestId}'`);

        return parsed as WebsocketResponse;
    }

    private connect(appId: string, node: Node) {
        let socket = new WebSocket(`ws://${node.ip_addr}:${node.api_port}/apps/${appId}/ws`);

        this.socket = socket;

        socket.onopen = () => {
            this.isOpen = true;
            console.log("websocket opened")
        };

        socket.onerror = (e) => {
            console.log("error: " + e)
        };

        socket.onmessage = (msg) => {
            console.log("AZAZAAAAAAAAAAAAAAAAA");
            console.log(msg);
            let response;
            try {
                let rawResponse = WebsocketSession.parseRawResponse(msg.data);

                console.log(rawResponse);

                if (rawResponse.type === "tx_wait_response") {

                    if (!this.waitingRequests.has(rawResponse.request_id)) {
                        console.log(`There is no message with requestId '${rawResponse.request_id}'`)
                    } else {
                        let executor = this.waitingRequests.get(rawResponse.request_id) as ResultExecutor;
                        if (rawResponse.type === "tx_wait_response") {
                            if (rawResponse.data) {
                                let parsed = JSON.parse(rawResponse.data).result.response;
                                let result = new Result(toByteArray(parsed.value));

                                executor.resultHandler(result)
                            }
                            if (rawResponse.error) {
                                executor.errorHandler(rawResponse.error as string);
                            }
                        } else if (rawResponse.type === "subscribe_response") {

                        }
                        if (!executor.subscription) {
                            this.waitingRequests.delete(rawResponse.request_id);
                        }
                    }
                } else {

                }
            } catch (e) {
                console.log("Cannot parse websocket event: " + e)
            }
        };

        socket.onclose = (e) => {
            this.isOpen = false;
            console.log("websocket closed " + e)
        }
    }
}
