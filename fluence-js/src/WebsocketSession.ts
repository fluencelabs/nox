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
import {Executor, PromiseExecutor, SubscribtionExecutor} from "./executor";

interface WebsocketResponse {
    request_id: string
    type: string
    data?: string
    error?: string
}

export class WebsocketSession {
    private sessionId: string;
    private appId: string;
    private readonly privateKey?: PrivateKey;
    private counter: number;
    private nodes: Node[];
    private nodeCounter: number;
    private socket: WebSocket;

    private waitingRequests = new Map<string, Executor<Result | void>>();

    private connectionHandler: PromiseExecutor<void>;

    /**
     * Create connected websocket.
     *
     */
    static create(appId: string, nodes: Node[], privateKey?: PrivateKey): Promise<WebsocketSession> {
        let ws = new WebsocketSession(appId, nodes, privateKey);
        return ws.connect();
    }

    private constructor(appId: string, nodes: Node[], privateKey?: PrivateKey) {
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
    }

    private messageHandler(msg: string) {
        let response;
        try {
            let rawResponse = WebsocketSession.parseRawResponse(msg);

            console.log(rawResponse);

            if (!this.waitingRequests.has(rawResponse.request_id)) {
                console.log(`There is no message with requestId '${rawResponse.request_id}'`)
            } else {
                if (rawResponse.type === "tx_wait_response") {
                    let executor = this.waitingRequests.get(rawResponse.request_id) as Executor<Result>;
                    if (rawResponse.data) {
                        let parsed = JSON.parse(rawResponse.data).result.response;
                        let result = new Result(toByteArray(parsed.value));

                        executor.handleResult(result)
                    } else if (rawResponse.error) {
                        executor.handleError(rawResponse.error as string);
                    }
                    if (executor.type === "promise") {
                        this.waitingRequests.delete(rawResponse.request_id);
                    }
                } else {
                    let executor = this.waitingRequests.get(rawResponse.request_id) as Executor<void>;
                    executor.handleResult()
                }
            }


        } catch (e) {
            console.log("Cannot parse websocket event: " + e)
        }
    }

    /**
     * Creates a new websocket connection. Waits after websocket will become connected.
     */
    private connect(): Promise<WebsocketSession> {
        let node = this.nodes[this.nodeCounter % this.nodes.length];
        this.nodeCounter++;
        this.connectionHandler = new PromiseExecutor<void>();

        let socket = new WebSocket(`ws://${node.ip_addr}:${node.api_port}/apps/${this.appId}/ws`);

        this.socket = socket;

        socket.onopen = () => {
            console.log("websocket opened");
            this.connectionHandler.handleResult()
        };

        socket.onerror = (e) => {
            this.reconnectSession(e)
        };

        socket.onclose = (e) => {
            this.reconnectSession(e)
        };

        socket.onmessage = (msg) => {
            this.messageHandler(msg.data)

        };

        return this.connectionHandler.promise().then(() => this);
    }

    /**
     * Increments current internal counter
     */
    private getCounterAndIncrement() {
        return this.counter++;
    }

    /**
     * Delete a subscription.
     *
     */
    async unsubscribe(subscriptionId: string): Promise<void> {

        await this.connectionHandler.promise();

        let requestId = genRequestId();

        let request = {
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "unsubscribe_request"
        };

        await this.sendAndWaitResponse(requestId, JSON.stringify(request));

        this.waitingRequests.delete(subscriptionId);
    }

    /**
     * Creates a subscription, that will return responses on every change in a state machine.
     * @param transaction will be run on state machine on every change
     * @param resultHandler to handle changes
     * @param errorHandler to handle errors
     */
    async subscribe(transaction: string, resultHandler: (result: Result) => void, errorHandler: (error: any) => void): Promise<string> {
        await this.connectionHandler.promise();
        let requestId = genRequestId();
        let subscriptionId = genRequestId();

        let request = {
            tx: transaction,
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "subscribe_request"
        };

        let promise = this.sendAndWaitResponse(requestId, JSON.stringify(request));
        await promise;

        let executor: SubscribtionExecutor = new SubscribtionExecutor(resultHandler, errorHandler);

        this.waitingRequests.set(subscriptionId, executor);

        return promise.then(() => subscriptionId);
    }

    /**
     *
     * @param payload
     */
    async requestAsync(payload: string): Promise<void> {

        await this.connectionHandler.promise();

        let requestId = genRequestId();
        let counter = this.getCounterAndIncrement();

        let tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        let request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_request"
        };

        return this.sendAndWaitResponse(requestId, JSON.stringify(request)).then((r) => {});
    }

    request(payload: string): Promise<Result> {
        let requestId = genRequestId();
        let counter = this.getCounterAndIncrement();

        let tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        let request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_wait_request"
        };

        console.log("send request: " + JSON.stringify(request));

        return this.sendAndWaitResponse(requestId, JSON.stringify(request))
    }

    private sendAndWaitResponse(requestId: string, message: string): Promise<Result> {
        this.socket.send(message);

        let executor: PromiseExecutor<Result> = new PromiseExecutor();

        this.waitingRequests.set(requestId, executor);

        return executor.promise()
    }

    private reconnectSession(reason: any) {
        this.sessionId = genSessionId();
        this.counter = 0;
        this.connectionHandler.handleError(reason);
        this.connect();
        this.waitingRequests.forEach((value: Executor<Result>, key: string) => {
            value.handleError("Reconnecting. All waiting requests are terminated.")
        });
        this.waitingRequests.clear();
    }

    private static parseRawResponse(response: string): WebsocketResponse {
        let parsed = JSON.parse(response);
        if (!parsed.request_id) throw new Error("Cannot parse response, no 'request_id' field.");
        if (parsed.type === "tx_wait_response" && !parsed.data && !parsed.error) throw new Error(`Cannot parse response, no 'data' or 'error' field in response with requestId '${parsed.requestId}'`);

        return parsed as WebsocketResponse;
    }
}
