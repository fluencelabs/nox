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
import {Result} from "./Result";
import {Executor, ExecutorType, PromiseExecutor, SubscribtionExecutor} from "./executor";
import {debug, TendermintClient} from "./fluence";
import {AbciQueryResult, TendermintJsonRpcResponse} from "./RpcClient";
import {none} from "ts-option";

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
    private timeout: number;
    private socket: WebSocket;
    private firstConnection: boolean = true;

    // result is for 'txWaitRequest' and 'query', void is for all other requests
    private executors = new Map<string, Executor<Result | void>>();

    // promise, that should be completed if websocket is connected
    private connectionHandler: PromiseExecutor<void>;

    /**
     * Create connected websocket.
     *
     */
    static create(appId: string, nodes: Node[], privateKey?: PrivateKey, timeout = 15000): Promise<WebsocketSession> {
        const ws = new WebsocketSession(appId, nodes, timeout, privateKey);
        return ws.connect();
    }

    private constructor(appId: string, nodes: Node[], timeout: number, privateKey?: PrivateKey) {
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
        this.timeout = timeout;
    }

    private messageHandler(msg: string) {
        let response;
        try {
            const rawResponse = WebsocketSession.parseRawResponse(msg);

            debug("Message received: " + JSON.stringify(rawResponse));

            if (!this.executors.has(rawResponse.request_id)) {
                console.error(`There is no message with requestId '${rawResponse.request_id}'. Message: ${msg}`)
            } else {
                const executor = this.executors.get(rawResponse.request_id) as Executor<Result>;
                if (rawResponse.error) {
                    console.log(`Error received for ${rawResponse.request_id}: ${JSON.stringify(rawResponse.error)}`);
                    executor.fail(rawResponse.error as string);
                } else if (rawResponse.type === "tx_wait_response") {
                    if (rawResponse.data) {
                        const parsed = JSON.parse(rawResponse.data) as TendermintJsonRpcResponse<AbciQueryResult>;
                        const result = TendermintClient.parseQueryResponse(none, parsed);

                        if (result.isEmpty) {
                            console.error(`Unexpected, no parsed result in message: ${msg}`)
                        } else {
                            executor.complete(result.get)
                        }
                    }
                    if (Executor.isPromise(executor)) {
                        this.executors.delete(rawResponse.request_id);
                        executor.cancelTimeout();
                    }
                } else {
                    const executor = this.executors.get(rawResponse.request_id) as Executor<void>;
                    executor.complete()
                }
            }


        } catch (e) {
            console.error("Cannot parse websocket event: " + e)
        }
    }

    /**
     * Trying to subscribe to all existed subscriptions again.
     */
    private resubscribe() {
        this.executors.forEach((executor: Executor<any>, key: string) => {
            if (executor.type === ExecutorType.Subscription) {
                const subExecutor = executor as SubscribtionExecutor;
                this.subscribe(subExecutor.subscription, subExecutor.resultCallback, subExecutor.errorCallback)
                    .catch((e) => console.error(`Cannot resubscribe on ${subExecutor.subscription}`))
            }
        });
    }

    /**
     * Creates a new websocket connection. Waits after websocket will become connected.
     */
    private connect(): Promise<WebsocketSession> {
        const node = this.nodes[this.nodeCounter % this.nodes.length];
        this.nodeCounter++;
        debug("Websocket connecting to " + JSON.stringify(node));

        if (!this.connectionHandler || !this.firstConnection) {
            this.connectionHandler = new PromiseExecutor<void>(undefined);
        }

        try {
            const socket = new WebSocket(`ws://${node.ip_addr}:${node.api_port}/apps/${this.appId}/ws`);

            this.socket = socket;

            socket.onopen = () => {
                debug("Websocket is opened");
                this.firstConnection = false;
                this.connectionHandler.complete();
                this.resubscribe();
            };

            socket.onerror = (e) => {
                console.error("Websocket receive an error: " + JSON.stringify(e) + ". Reconnecting on close.");
            };

            socket.onclose = (e) => {
                console.error("Websocket is closed. Reconnecting.");

                // new requests will be terminated until websocket is connected
                // TODO: fail first connection if all nodes are unavailable
                if (!this.firstConnection) {
                    this.connectionHandler = new PromiseExecutor<void>(undefined);
                    this.connectionHandler.fail("Websocket is closed. Reconnecting")
                }

                this.stopPromiseExecutors();

                setTimeout(() => this.reconnectSession(e), 1000)
            };

            socket.onmessage = (msg) => this.messageHandler(msg.data);
        } catch (e) {
            console.log("Websocket error on connecting: " + JSON.stringify(e));
        }
        return this.connectionHandler.promise.then(() => this);
    }

    private stopPromiseExecutors() {
        // terminate and delete all promise executors
        this.executors.forEach((executor: Executor<Result | void>, key: string) => {
            if (executor.type === ExecutorType.Promise) {
                executor.fail("Reconnecting. All waiting requests are terminated.");
                this.executors.delete(key)

            }
        });
    }

    /**
     * Increments current internal counter.
     */
    private getCounterAndIncrement() {
        return this.counter++;
    }

    /**
     * Delete a subscription.
     *
     */
    async unsubscribe(subscriptionId: string): Promise<void> {

        debug("Unsibscribe " + subscriptionId);

        await this.connectionHandler.promise;

        const requestId = genRequestId();

        const request = {
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "unsubscribe_request"
        };

        await this.sendAndWaitResponse(requestId, JSON.stringify(request));

        this.executors.delete(subscriptionId);
    }

    private async subscribeCall(transaction: string, requestId: string, subscriptionId: string): Promise<Result> {
        const request = {
            tx: transaction,
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "subscribe_request"
        };

        return this.sendAndWaitResponse(requestId, JSON.stringify(request));
    }

    /**
     * Creates a subscription, that will return responses on every change in a state machine.
     * @param transaction will be run on state machine on every change
     * @param resultHandler to handle changes
     * @param errorHandler to handle errors
     */
    async subscribe(transaction: string, resultHandler: (result: Result) => void, errorHandler: (error: any) => void): Promise<string> {
        await this.connectionHandler.promise;
        const requestId = genRequestId();
        const subscriptionId = genRequestId();

        const executor: SubscribtionExecutor = new SubscribtionExecutor(transaction, resultHandler, errorHandler);

        await this.subscribeCall(transaction, requestId, subscriptionId);

        this.executors.set(subscriptionId, executor);

        return subscriptionId
    }

    /**
     * Send a request without waiting a response.
     */
    async requestAsync(payload: string): Promise<void> {

        await this.connectionHandler.promise;

        const requestId = genRequestId();
        const counter = this.getCounterAndIncrement();

        const tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        const request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_request"
        };

        return this.sendAndWaitResponse(requestId, JSON.stringify(request)).then((r) => {});
    }

    /**
     * Send a request and waiting for a response.
     */
    request(payload: string): Promise<Result> {
        const requestId = genRequestId();
        const counter = this.getCounterAndIncrement();

        const tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        const request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_wait_request"
        };

        console.log("send request: " + JSON.stringify(request));

        return this.sendAndWaitResponse(requestId, JSON.stringify(request))
    }

    /**
     * Send a request to websocket and create a promise that will wait for a response.
     */
    private sendAndWaitResponse(requestId: string, message: string): Promise<Result> {
        this.socket.send(message);

        const timeout = setTimeout(() => {
            if (this.executors.has(requestId)) {
                executor.fail(`Timeout after ${this.timeout} milliseconds.`);
                this.executors.delete(requestId);
            }
        }, this.timeout);

        const executor: PromiseExecutor<Result> = new PromiseExecutor(timeout);

        this.executors.set(requestId, executor);

        return executor.promise
    }

    /**
     * Generate new sessionId, terminate old connectionHandler and create a new one.
     * Terminate all executors that are waiting for responses.
     */
    private reconnectSession(reason: any) {
        this.sessionId = genSessionId();
        this.counter = 0;
        this.connect();
    }

    private static parseRawResponse(response: string): WebsocketResponse {
        const parsed = JSON.parse(response);
        if (!parsed.request_id) throw new Error("Cannot parse response, no 'request_id' field.");
        if (parsed.type === "tx_wait_response" && !parsed.data && !parsed.error) throw new Error(`Cannot parse response, no 'data' or 'error' field in response with requestId '${parsed.requestId}'`);

        return parsed as WebsocketResponse;
    }
}
