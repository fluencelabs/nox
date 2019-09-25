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
import {none} from "ts-option";

interface WebsocketResponse {
    request_id: string
    type: string
    data?: string
    error?: string
}

/**
 * Session is a stable stateful asynchronous connection to Fluence application.
 * Session manages websocket connections to nodes.
 * Controls the order of requests.
 * It tries to connect to one node at a time.
 * First connection will fail only if all nodes are unavailable.
 * Creates new sessionId on each node.
 * If request becomes failed due to timeout, a connection to the next node is created,
 * because the client doesn't know if a request processed on a node or not.
 * All pending requests (request and requestAsync methods) will fail after reconnecting.
 * If all nodes are unavailable, requests will fail, the session will try to reconnect to nodes after a timeout.
 * Keeps subscriptions (subscribe method) after each reconnects.
 */
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

    // result is for 'request' and 'requestAsync', void is for all other requests
    private executors = new Map<string, Executor<any | void>>();

    // promise, that should be completed if websocket is connected
    private connectionPromise: PromiseExecutor<void>;

    /**
     * Create connected websocket.
     *
     * @param appId id of an application
     * @param nodes list of nodes to connect
     * @param privateKey key that will sign all requests
     * @param timeout time after which the request fails
     *
     * @return promise that will be completed if websocket become connected and will fail if all nodes will be unavailable
     */
    static create(appId: string, nodes: Node[], privateKey?: PrivateKey, timeout = 15000): Promise<WebsocketSession> {
        const ws = new WebsocketSession(appId, nodes, timeout, privateKey);
        return ws.connect();
    }

    private constructor(appId: string, nodes: Node[], timeout: number, privateKey?: PrivateKey) {
        if (nodes.length == 0) {
            console.error("There are no nodes to connect");
            throw new Error("There are no nodes to connect");
        }

        this.counter = 0;
        // first connection will be to the random node
        this.nodeCounter = Math.floor(Math.random() * nodes.length);
        this.sessionId = genSessionId();
        this.appId = appId;
        this.nodes = nodes;
        this.privateKey = privateKey;
        this.timeout = timeout;
        this.connectionPromise = new PromiseExecutor<void>();
    }

    /**
     * Creates a subscription, that will return responses on every change in Fluence application.
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

    /**
     * Send a request without waiting a response.
     *
     * @return promise, that completes if Fluence node will put request to a queue.
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
        return this.sendAndWaitResponse<BroadcastTxSyncResponse>(requestId, JSON.stringify(request)).then(() => {});
    }

    /**
     * Send a request and waiting for a response.
     */
    async request(payload: string): Promise<Result> {
        const requestId = genRequestId();
        const counter = this.getCounterAndIncrement();

        const tx = prepareRequest(payload, this.sessionId, counter, this.privateKey);

        const request = {
            tx: tx.payload,
            request_id: requestId,
            type: "tx_wait_request"
        };

        return this.sendAndWaitResponse<Result>(requestId, JSON.stringify(request))
    }

    /**
     * Creates a new websocket connection. Waits after websocket will become connected.
     */
    private connect(): Promise<WebsocketSession> {

        if (this.nodes.length === 0) {
            // all nodes are unavailable, return the error
            const errorMsg = "There are no available nodes";
            this.connectionPromise.fail(errorMsg);
            setTimeout(() => this.reconnectSession(errorMsg), 1000);
            return Promise.reject(errorMsg)
        }

        // chooses the next node after every connect
        const nodeNumber = this.nodeCounter % this.nodes.length;
        const node = this.nodes[nodeNumber];
        this.nodeCounter++;
        debug("Websocket connecting to " + JSON.stringify(node));

        if (this.connectionPromise.isFailed()) {
            this.connectionPromise = new PromiseExecutor<void>();
        }

        try {
            const socket = new WebSocket(`ws://${node.ip_addr}:${node.api_port}/apps/${this.appId}/ws`);

            this.socket = socket;

            socket.onopen = () => this.callbackOnOpen();

            socket.onerror = (e) => {
                console.error("Websocket receive an error: " + JSON.stringify(e) + ". Reconnecting on close.");
            };

            socket.onclose = (e) => this.callbackOnClose(node, nodeNumber, e);

            socket.onmessage = (msg) => this.messageHandler(msg.data);

        } catch (e) {
            console.log("Websocket error on connecting: " + JSON.stringify(e));
        }
        return this.connectionPromise.promise.then(() => this);
    }

    /**
     * Deletes node from the pool for a timeout, starts reconnecting.
     */
    private callbackOnClose(node: Node, nodeNumber: number, error: any) {
        console.error(`Websocket ${node.ip_addr}:${node.api_port} is closed. Reconnecting.`);

        // delete node from list of nodes and add it after timeout
        delete this.nodes[nodeNumber];
        setTimeout(() => this.nodes.push(node), this.timeout);

        this.failPendingRequests();

        setTimeout(() => this.reconnectSession(error), 1000);
    }

    /**
     * Completes connection promise and resubscribes on existing subscriptions.
     */
    private callbackOnOpen() {
        debug("Websocket is opened");
        this.reconnecting = false;
        this.connectionPromise.success();
        this.resubscribe();
    }

    private messageHandler(msg: string) {
        debug(`Message received: ${msg}`);
        try {
            const rawResponse = WebsocketSession.parseRawResponse(msg);

            debug("Websocket response: " + JSON.stringify(rawResponse));

            const executor = this.executors.get(rawResponse.request_id);
            if (!executor) {
                console.error(`There is no message with requestId '${rawResponse.request_id}'. Message: ${msg}`);
                return;
            }

            if (executor instanceof SubscriptionExecutor) {
                debug("Handle message for subscription: " + executor.subscription);
            }

            if (rawResponse.error) {
                console.log(`Error received for ${rawResponse.request_id}: ${JSON.stringify(rawResponse.error)}`);
                executor.fail(rawResponse.error);
            } else {
                this.parseData(rawResponse.data, rawResponse.type, rawResponse.request_id)
                    .then((r) => executor.success(r))
                    .catch((e) => {
                        console.error("Error on parsing data: " + e);
                        executor.fail(e)
                    })
            }
        } catch (e) {
            console.error("Cannot parse websocket event: " + e);
            console.error(e);
        }
    }

    private parseData(data: string | undefined, type: string, requestId: string): Promise<any | void> {
        if (!data) {
            debug("Resolving for " + requestId);
            // subscribe and unsubscribe requests
            return Promise.resolve()
        }
        if (type === "tx_wait_response") {
            const parsed = JSON.parse(data) as TendermintJsonRpcResponse<AbciQueryResult>;
            const result = TendermintClient.parseQueryResponse(none, parsed);

            return result
                .fold(
                    () => {
                        const errorMessage = `Unexpected, no parsed result in message: ${data}`;
                        console.error(errorMessage);
                        return Promise.reject<Result>(errorMessage);
                    }
                )((r) => {
                    debug("Result received: " + r.asString());
                    return Promise.resolve(r)
                })
        } else if (type === "tx_response") {
            const response = parseResponse(JSON.parse(data));
            if (response.code !== 0) {
                return Promise.reject(`There is an error on request '${requestId}': ${JSON.stringify(response)}`)
            } else {
                return Promise.resolve(response)
            }
        } else {
            return Promise.reject(`Unexpected type of message ${type}. RequestId: ${requestId}, data: ${data}`)
        }
    }

    /**
     * If timeout occurred, delete executor from state and do reconnect.
     *
     */
    private onTimeout<T>(requestId: string, message: string) {
        const executor = this.executors.get(requestId);
        if (executor) {
            executor.fail(`Timeout after ${this.timeout} milliseconds. requestId: ${requestId}, msg: ${message}`);
            if (!this.reconnecting) {
                this.reconnecting = true;
                // close current session, it will be reconnected in weboscket `onclose` callback
                // all pending transactions will be failed
                this.socket.close(4000, "Timeout occurred. Reconnecting to another node.");
            }
        }
    }

    /**
     * Send a request to websocket and create a promise that will wait for a response.
     */
    private sendAndWaitResponse<T>(requestId: string, message: string): Promise<T> {
        debug(`Sending message ${message}`);
        this.socket.send(message);

        const onTimeout = () => {
            this.onTimeout(requestId, message)
        };

        const onComplete = () => {
            this.executors.delete(requestId);
        };

        const executor: PromiseExecutor<T> = new PromiseExecutor<T>(onComplete, onTimeout, this.timeout);
        this.executors.set(requestId, executor);
        return executor.promise
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
                    .catch((e) => console.error(`Cannot resubscribe on ${subExecutor.subscription}. Error: ${e}`))
            }
        });
    }

    private failPendingRequests() {
        // terminate all promise executors
        this.executors.forEach((executor: Executor<Result | void>) => {
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
     * Send a message about new subscription.
     */
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
     * Generate new sessionId, terminate old connectionPromise and create a new one.
     * Terminate all executors that are waiting for responses.
     */
    private reconnectSession(reason: any): Promise<WebsocketSession> {
        this.sessionId = genSessionId();
        this.counter = 0;
        return this.connect();
    }

    private static parseRawResponse(response: string): WebsocketResponse {
        const parsed = JSON.parse(response);
        if (!parsed.request_id) throw new Error("Cannot parse response, no 'request_id' field.");
        if (parsed.type === "tx_wait_response" && !parsed.data && !parsed.error) throw new Error(`Cannot parse response, no 'data' or 'error' field in response with requestId '${parsed.requestId}'`);

        return parsed as WebsocketResponse;
    }
}
