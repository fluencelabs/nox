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

import {genMessage, networkStateMessage} from "./messages";

export let debug = require('debug')('janus');
export let debugI = require('debug');

// debug logs are disabled by default
debugI.disable();

export function enableDebug() {
    debugI.enable("fluence");
}

export function disableDebug() {
    debugI.disable();
}

export class JanusConnection {

    readonly socket: WebSocket;
    private connected: boolean;

    constructor(peerId: string, host?: string, port?: number) {
        if (!port) port = 9999;
        if (!host) host = "localhost";
        this.socket = new WebSocket(`ws://${host}:${port}/ws?key=${peerId}`);
        this.socket.onopen = (ev: Event) => {
            this.connected = true;
        };

        this.socket.onclose = (ev: CloseEvent) => {
            this.connected = false;
            console.log("Connection closed: " + JSON.stringify(ev.reason))
        };

        this.socket.onmessage = (ev: MessageEvent) => {
            console.log(ev.data);
        };
    }

    /**
     * Sends a message to another peer through janus nodes.
     * @param destination peer_id of a receiver
     * @param message
     */
    public relayMessage(destination: string, message: string) {
        if (!this.connected) {
            console.log("Connection is not established.");
            return;
        }

        this.socket.send(JSON.stringify(genMessage(destination, message)));
    }

    /**
     * Gets state of the janus network.
     */
    public getNetworkState() {
        if (!this.connected) {
            console.log("Connection is not established.");
            return;
        }

        this.socket.send(JSON.stringify(networkStateMessage()));
    }
}

/**
 * Connects to a janus node.
 * @param peerId in libp2p format. Example:
 *                                      QmUz5ziqFiwuPJnUZehrQ3EyzpHjp22FyQRNH9AxRxKPbp
 *                                      QmcYE4o3HCpotey8Xm87ArERDp9KMgagUnjtKBxuA5vcBY
 * @param host localhost by default
 * @param port 9999 by default
 */
export function connect(peerId: string, host?: string, port?: number): JanusConnection {
    return new JanusConnection(peerId, host, port)
}

(<any>window).connect = connect;
