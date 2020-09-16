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

import {Address, createPeerAddress} from "./address";
import {
    callToString,
    FunctionCall,
    genUUID,
    makeFunctionCall,
    makeProvideMessage,
    parseFunctionCall
} from "./functionCall";

import Websockets from "libp2p-websockets";
import Mplex from "libp2p-mplex";
import SECIO from "libp2p-secio";
import Peer from "libp2p";
import {decode, encode} from "it-length-prefixed";
import pipe from "it-pipe";
import Multiaddr from "multiaddr";
import PeerId from "peer-id";
import * as log from 'loglevel';

export const PROTOCOL_NAME = '/fluence/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class FluenceConnection {

    private readonly selfPeerId: PeerId;
    readonly sender: Address;
    private node: LibP2p;
    private readonly address: Multiaddr;
    readonly nodePeerId: PeerId;
    private readonly selfPeerIdStr: string;
    private readonly handleCall: (call: FunctionCall) => FunctionCall | undefined;

    constructor(multiaddr: Multiaddr, hostPeerId: PeerId, selfPeerId: PeerId, sender: Address, handleCall: (call: FunctionCall) => FunctionCall | undefined) {
        this.selfPeerId = selfPeerId;
        this.handleCall = handleCall;
        this.selfPeerIdStr = selfPeerId.toB58String();
        this.address = multiaddr;
        this.nodePeerId = hostPeerId;
        this.sender = sender
    }

    makeReplyTo(reply?: string): Address {
        if (reply) {
            let replyToWithHash = {...this.sender}
            if (typeof reply === "string") replyToWithHash.hash = reply;
            return replyToWithHash;
        } else {
            return this.sender;
        }
    }

    async connect() {
        let peerInfo = this.selfPeerId;
        this.node = await Peer.create({
            peerId: peerInfo,
            config: {},
            modules: {
                transport: [Websockets],
                streamMuxer: [Mplex],
                connEncryption: [SECIO],
                peerDiscovery: []
            },
        });

        await this.startReceiving();
    }

    isConnected() {
        return this.status === Status.Connected
    }

    // connection status. If `Disconnected`, it cannot be reconnected
    private status: Status = Status.Initializing;

    private async startReceiving() {
        if (this.status === Status.Initializing) {
            await this.node.start();

            log.debug("dialing to the node with address: " + this.node.peerId.toB58String());

            await this.node.dial(this.address);

            let _this = this;

            this.node.handle([PROTOCOL_NAME], async ({connection, stream}) => {
                pipe(
                    stream.source,
                    decode(),
                    async function (source: AsyncIterable<string>) {
                        for await (const msg of source) {
                            try {
                                log.debug(_this.selfPeerIdStr);
                                let call = parseFunctionCall(msg);
                                let response = _this.handleCall(call);

                                // send a response if it exists, do nothing otherwise
                                if (response) {
                                    await _this.sendCall(response);
                                }
                            } catch(e) {
                                log.error("error on handling a new incoming message: " + e);
                            }
                        }
                    }
                )
            });

            this.status = Status.Connected;
        } else {
            throw Error(`can't start receiving. Status: ${this.status}`);
        }
    }

    private checkConnectedOrThrow() {
        if (this.status !== Status.Connected) {
            throw Error(`connection is in ${this.status} state`)
        }
    }

    async disconnect() {
        await this.node.stop();
        this.status = Status.Disconnected;
    }

    private async sendCall(call: FunctionCall) {
        let callStr = callToString(call);
        log.debug("send function call: " + JSON.stringify(JSON.parse(callStr), undefined, 2));
        log.debug(call);

        // create outgoing substream
        const conn = await this.node.dialProtocol(this.address, PROTOCOL_NAME) as {stream: Stream; protocol: string};

        pipe(
            [callStr],
            // at first, make a message varint
            encode(),
            conn.stream.sink,
        );
    }

    /**
     * Send FunctionCall to the connected node.
     */
    async sendFunctionCall(target: Address, args: any, moduleId?: string, fname?: string, msgId?: string, name?: string) {
        this.checkConnectedOrThrow();

        let replyTo;
        if (msgId) replyTo = this.makeReplyTo(msgId);

        let call = makeFunctionCall(genUUID(), target, this.sender, args, moduleId, fname, replyTo, name);

        await this.sendCall(call);
    }

    async provideName(name: string) {
        let target = createPeerAddress(this.nodePeerId.toB58String())
        let regMsg = await makeProvideMessage(name, target, this.sender);
        await this.sendCall(regMsg);
    }
}
