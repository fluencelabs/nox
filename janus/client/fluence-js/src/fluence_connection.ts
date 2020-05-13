/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

import {Address} from "./address";
import {
    callToString,
    FunctionCall,
    genUUID,
    makeCall,
    makeFunctionCall,
    makePeerCall,
    makeRegisterMessage,
    makeRelayCall,
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

export const PROTOCOL_NAME = '/janus/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class FluenceConnection {

    private readonly selfPeerInfo: PeerInfo;
    readonly replyToAddress: Address;
    private node: LibP2p;
    private readonly address: Multiaddr;
    private readonly nodePeerId: PeerId;
    private readonly selfPeerId: string;
    private readonly handleCall: (call: FunctionCall) => FunctionCall | undefined;

    constructor(multiaddr: Multiaddr, hostPeerId: PeerId, selfPeerInfo: PeerInfo, replyToAddress: Address, handleCall: (call: FunctionCall) => FunctionCall | undefined) {
        this.selfPeerInfo = selfPeerInfo;
        this.handleCall = handleCall;
        this.selfPeerId = selfPeerInfo.id.toB58String();
        this.address = multiaddr;
        this.nodePeerId = hostPeerId;
        this.replyToAddress = replyToAddress
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

    /**
     * Sends remote service_id call.
     */
    async sendServiceCall(serviceId: string, args: any, name?: string) {
        let regMsg = makeCall(serviceId, args, this.replyToAddress, name);
        await this.sendCall(regMsg);
    }

    /**
     * Sends custom message to the peer.
     */
    async sendPeerCall(peer: string, msg: any, name?: string) {
        let regMsg = makePeerCall(PeerId.createFromB58String(peer), msg, this.replyToAddress, name);
        await this.sendCall(regMsg);
    }

    /**
     * Sends custom message to the peer through relay.
     */
    async sendRelayCall(peer: string, relay: string, msg: any, name?: string) {
        let regMsg = await makeRelayCall(PeerId.createFromB58String(peer), PeerId.createFromB58String(relay), msg, this.replyToAddress, name);
        await this.sendCall(regMsg);
    }

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
        console.log("send function call: " + callStr);
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
    async sendFunctionCall(target: Address, args: any, reply?: boolean, name?: string) {
        this.checkConnectedOrThrow();

        let replyTo;
        if (reply) replyTo = this.replyToAddress;

        let call = makeFunctionCall(genUUID(), target, args, replyTo, name);

        await this.sendCall(call);
    }

    async registerService(serviceId: string) {
        let regMsg = await makeRegisterMessage(serviceId, this.nodePeerId, this.selfPeerInfo.id);
        await this.sendCall(regMsg);
    }
}