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
import {Executor, ExecutorType, PromiseExecutor, SubscriptionExecutor} from "./executor";
import {BroadcastTxSyncResponse, parseResponse} from "./TendermintClient";
import {debug, TendermintClient} from "./fluence";
import {AbciQueryResult, TendermintJsonRpcResponse} from "./RpcClient";
import {none, Option} from "ts-option";

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
    private reconnecting: boolean;

    // on first connection `connectionPromise` will fail only if all nodes are unavailable
    private firstConnection: boolean = true;

    // result is for 'txWaitRequest' and 'query', void is for all other requests
    private executors = new Map<string, Executor<any | void>>();

    // promise, that should be completed if websocket is connected
    private connectionPromise: PromiseExecutor<void>;

    /**
     * Create connected websocket.
     * @param appId id of an application
     * @param nodes list of nodes to connect
     * @param privateKey key that will sign all requests
     * @param timeout time after which the request fails
     */
    static create(appId: string, nodes: Node[], privateKey?: PrivateKey, timeout = 10000): Promise<WebsocketSession> {
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

    private static handleResult(r: string, executor: Executor<any>): void {
        const result = WebsocketSession.parseResponse(r);
        if (executor instanceof SubscriptionExecutor) {
            debug(executor.subscription);
        }

        return result
            .fold(
                () => {
                    const errorMessage = `Unexpected, no parsed result in message: ${r}`;
                    console.error(errorMessage);
                    executor.fail(errorMessage);
                }
            )((r) => {
                debug("Result received: " + r.asString());
                executor.success(r)
            })
    }

    private static handleTxResponse(r: string, requestId: string, executor: Executor<any>): void {
        const response = parseResponse(JSON.parse(r));
        if (response.code !== 0) {
            executor.fail(`There is an error on request '${requestId}': ${JSON.stringify(response)}`)
        } else {
            executor.success(response)
        }
    }

    private messageHandler(msg: string) {
        debug(`Message received: ${msg}`);
        try {
            const rawResponse = WebsocketSession.parseRawResponse(msg);

            debug("Message received: " + JSON.stringify(rawResponse));

            const executor = this.executors.get(rawResponse.request_id);
            if (!executor) {
                console.error(`There is no message with requestId '${rawResponse.request_id}'. Message: ${msg}`);
                return;
            }

            if (rawResponse.error) {
                console.log(`Error received for ${rawResponse.request_id}: ${JSON.stringify(rawResponse.error)}`);
                executor.fail(rawResponse.error);
            } else if (rawResponse.type === "tx_wait_response") {
                WebsocketSession.handleResult(rawResponse.data as string, executor)
            } else if (rawResponse.type === "tx_response") {
                WebsocketSession.handleTxResponse(rawResponse.data as string, rawResponse.request_id, executor)
            } else {
                // subscribe and unsubscribe requests
                executor.success({})
            }
        } catch (e) {
            console.error("Cannot parse websocket event: " + e);
            console.error(e);
        }
    }

    private static parseResponse(data: string): Option<Result> {
        const parsed = JSON.parse(data) as TendermintJsonRpcResponse<AbciQueryResult>;
        return TendermintClient.parseQueryResponse(none, parsed);
    }

    /**
     * Trying to subscribe to all existed subscriptions again.
     */
    private resubscribe() {
        this.executors.forEach((executor: Executor<any>, key: string) => {
            if (executor.type === ExecutorType.Subscription) {
                const subExecutor = executor as SubscriptionExecutor;
                debug("Resubscribe: " + subExecutor.subscription);
                const requestId = genRequestId();
                this.subscribeCall(subExecutor.subscription, requestId, key)
                    .catch((e) => console.error(`Cannot resubscribe on ${subExecutor.subscription}`))
            }
        });
    }

    /**
     * Creates a new websocket connection. Waits after websocket will become connected.
     */
    private connect(): Promise<WebsocketSession> {

        if (this.nodes.length === 0) {
            // all nodes are unavailable, return the error
            if (this.connectionPromise) {
                this.connectionPromise.fail("There is no available nodes.")
            }
            return Promise.reject("There is no available nodes.")
        }

        const nodeNumber = this.nodeCounter % this.nodes.length;
        const node = this.nodes[nodeNumber];
        this.nodeCounter++;
        debug("Websocket connecting to " + JSON.stringify(node));

        if (!this.connectionPromise || !this.firstConnection) {
            this.connectionPromise = new PromiseExecutor<void>();
        }

        try {
            const socket = new WebSocket(`ws://${node.ip_addr}:${node.api_port}/apps/${this.appId}/ws`);

            this.socket = socket;

            socket.onopen = () => {
                debug("Websocket is opened");
                this.firstConnection = false;
                this.reconnecting = false;
                this.connectionPromise.success();
                this.resubscribe();
            };

            socket.onerror = (e) => {
                console.error("Websocket receive an error: " + JSON.stringify(e) + ". Reconnecting on close.");
            };

            socket.onclose = (e) => {
                console.error(`Websocket ${node.ip_addr}:${node.api_port} is closed. Reconnecting.`);

                // delete node from list of nodes and add it after timeout
                delete this.nodes[nodeNumber];
                setTimeout(() => this.nodes.push(node), this.timeout);

                // new requests will be terminated until websocket is connected
                if (!this.firstConnection) {
                    this.connectionPromise = new PromiseExecutor<void>();
                    this.connectionPromise.fail("Websocket is closed. Reconnecting")
                }

                this.stopPromiseExecutors();

                setTimeout(() => this.reconnectSession(e), 1000)
            };

            socket.onmessage = (msg) => this.messageHandler(msg.data);
        } catch (e) {
            console.log("Websocket error on connecting: " + JSON.stringify(e));
        }
        return this.connectionPromise.promise.then(() => this);
    }

    private stopPromiseExecutors() {
        // terminate and all promise executors
        this.executors.forEach((executor: Executor<Result | void>, key: string) => {
            if (executor.type === ExecutorType.Promise) {
                executor.fail("Reconnecting. All waiting requests are terminated.");
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

        await this.connectionPromise.promise;

        const requestId = genRequestId();

        const request = {
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "unsubscribe_request"
        };

        await this.sendAndWaitResponse<void>(requestId, JSON.stringify(request));

        this.executors.delete(subscriptionId);
    }

    private async subscribeCall(transaction: string, requestId: string, subscriptionId: string): Promise<void> {
        const request = {
            tx: transaction,
            request_id: requestId,
            subscription_id: subscriptionId,
            type: "subscribe_request"
        };

        return this.sendAndWaitResponse<void>(requestId, JSON.stringify(request));
    }

    /**
     * Creates a subscription, that will return responses on every change in a state machine.
     * @param transaction will be run on state machine on every change
     * @param resultCallback to handle changes
     * @param errorCallback to handle errors
     */
    async subscribe(transaction: string, resultCallback: (result: Result) => void, errorCallback: (error: any) => void): Promise<string> {
        await this.connectionPromise.promise;
        const requestId = genRequestId();
        const subscriptionId = genRequestId();

        const executor: SubscriptionExecutor = new SubscriptionExecutor(transaction, resultCallback, errorCallback);
        this.executors.set(subscriptionId, executor);

        await this.subscribeCall(transaction, requestId, subscriptionId);

        return subscriptionId
    }

    /**
     * Send a request without waiting a response.
     */
    async requestAsync(payload: string): Promise<void> {

        await this.connectionPromise.promise;

        const requestId = genRequestId();
        const counter = this.getCounterAndIncrement();

        const tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        const request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_request"
        };

        // if a promise is succeeded, a request was delivered, so we don't need a result
        return this.sendAndWaitResponse<BroadcastTxSyncResponse>(requestId, JSON.stringify(request)).then((r) => {});
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

        return this.sendAndWaitResponse<Result>(requestId, JSON.stringify(request))
    }

    /**
     * Send a request to websocket and create a promise that will wait for a response.
     */
    private sendAndWaitResponse<T>(requestId: string, message: string): Promise<T> {
        debug(`Sending message ${message}`);
        this.socket.send(message);

        // if timeout occurred, delete executor from state and do reconnect
        const onTimeout = () => {
            if (this.executors.has(requestId)) {
                executor.fail(`Timeout after ${this.timeout} milliseconds.`);
                this.executors.delete(requestId);
                if (!this.reconnecting) {
                    this.reconnecting = true;
                    // close current session, it will be reconnected in weboscket `onclose` callback
                    this.socket.close(4000, "Timeout occurred. Reconnecting to another node.");
                }
            }
        };

        const onComplete = () => {
            this.executors.delete(requestId);
        };

        const executor: PromiseExecutor<T> = new PromiseExecutor<T>(onComplete, onTimeout, this.timeout);

        this.executors.set(requestId, executor);

        return executor.promise
    }

    /**
     * Generate new sessionId, terminate old connectionPromise and create a new one.
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
