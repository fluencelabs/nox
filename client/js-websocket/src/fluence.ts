/*
 * Copyright 2020 Fluence Labs Limited
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
import nacl, {SignKeyPair} from "tweetnacl";
let wasm_utils: { decode(s: string): Uint8Array; encode(data: Uint8Array): string; sha3_256(input: Uint8Array): Uint8Array; keccak_256(input: Uint8Array): Uint8Array };

(async () => {
    wasm_utils = await import("../utils-wasm/pkg");
})();



export let debug = require('debug')('fluence');
export let debugI = require('debug');

// debug logs are disabled by default
debugI.disable();

export function enableDebug() {
    debugI.enable("fluence");
}

export function disableDebug() {
    debugI.disable();
}

export class FluenceConnection {

    readonly socket: WebSocket;
    private connected: boolean;
    private kp: SignKeyPair;

    constructor(peerId: string, host?: string, port?: number, seed?: string) {

        if (seed) {
            this.kp = nacl.sign.keyPair.fromSeed(Uint8Array.from(Buffer.from(seed, 'hex')));
        } else {
            this.kp = nacl.sign.keyPair()
        }


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
     * Sends a message to another peer through fluence nodes.
     * @param destination peer_id of a receiver
     * @param message
     */
    public relayMessage(destination: string, message: string) {
        if (!this.connected) {
            console.log("Connection is not established.");
            return;
        }

        let hash = wasm_utils.sha3_256(new TextEncoder().encode(message));
        console.log(this.kp.publicKey.byteLength);
        let pkHex = wasm_utils.encode(this.kp.publicKey);

        let signature = wasm_utils.encode(nacl.sign(hash, this.kp.secretKey));

        this.socket.send(JSON.stringify(genMessage(destination, message, pkHex, signature)));
    }

    /**
     * Gets state of the fluence network.
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
 * Connects to a fluence node.
 * @param peerId in libp2p format. Example:
 *                                      QmUz5ziqFiwuPJnUZehrQ3EyzpHjp22FyQRNH9AxRxKPbp
 *                                      QmcYE4o3HCpotey8Xm87ArERDp9KMgagUnjtKBxuA5vcBY
 * @param host localhost by default
 * @param port 9999 by default
 */
export function connect(peerId: string, host?: string, port?: number): FluenceConnection {
    return new FluenceConnection(peerId, host, port)
}

(<any>window).connect = connect;
