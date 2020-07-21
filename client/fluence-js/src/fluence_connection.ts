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
} from "./function_call";
import * as PeerId from "peer-id";
import * as PeerInfo from "peer-info";
import Websockets from "libp2p-websockets";
import Mplex from "libp2p-mplex";
import SECIO from "libp2p-secio";
import Peer from "libp2p";
import {decode, encode} from "it-length-prefixed";
import pipe from "it-pipe";
import Multiaddr from "multiaddr";

export const PROTOCOL_NAME = '/fluence/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class FluenceConnection {

    private readonly selfPeerInfo: PeerInfo;
    readonly sender: Address;
    private node: LibP2p;
    private readonly address: Multiaddr;
    readonly nodePeerId: PeerId;
    private readonly selfPeerId: string;
    private readonly handleCall: (call: FunctionCall) => FunctionCall | undefined;

    constructor(multiaddr: Multiaddr, hostPeerId: PeerId, selfPeerInfo: PeerInfo, sender: Address, handleCall: (call: FunctionCall) => FunctionCall | undefined) {
        this.selfPeerInfo = selfPeerInfo;
        this.handleCall = handleCall;
        this.selfPeerId = selfPeerInfo.id.toB58String();
        this.address = multiaddr;
        this.nodePeerId = hostPeerId;
        this.sender = sender
    }

    makeReplyTo(reply: boolean | string): Address {
        if (reply) {
            let replyToWithHash = {...this.sender}
            if (typeof reply === "string") replyToWithHash.hash = reply;
            return replyToWithHash;
        } else {
            return this.sender;
        }
    }

    async connect() {
        let peerInfo = this.selfPeerInfo;
        this.node = await Peer.create({
            peerInfo,
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

            console.log("dialing to the node with address: " + this.node.peerInfo.id.toB58String());

            await this.node.dial(this.address);

            let _this = this;

            this.node.handle([PROTOCOL_NAME], async ({connection, stream}) => {
                pipe(
                    stream.source,
                    decode(),
                    async function (source: AsyncIterable<string>) {
                        for await (const msg of source) {
                            try {
                                console.log(_this.selfPeerId);
                                let call = parseFunctionCall(msg);
                                let response = _this.handleCall(call);

                                // send a response if it exists, do nothing otherwise
                                if (response) {
                                    await _this.sendCall(response);
                                }
                            } catch(e) {
                                console.log("error on handling a new incoming message: " + e);
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
        console.log("send function call: " + JSON.stringify(JSON.parse(callStr), undefined, 2));
        console.log(call);

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
    async sendFunctionCall(target: Address, args: any, moduleId?: string, fname?: string, reply?: boolean | string, context?: string[], name?: string) {
        this.checkConnectedOrThrow();

        let replyTo;
        if (reply) replyTo = this.makeReplyTo(reply);

        let call = makeFunctionCall(genUUID(), target, this.sender, args, moduleId, fname, replyTo, context, name);

        await this.sendCall(call);
    }

    async provideName(name: string) {
        let target = createPeerAddress(this.nodePeerId.toB58String())
        let regMsg = await makeProvideMessage(name, target, this.sender);
        await this.sendCall(regMsg);
    }
}
