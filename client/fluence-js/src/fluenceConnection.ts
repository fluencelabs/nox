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

import Websockets from "libp2p-websockets";
import Mplex from "libp2p-mplex";
import SECIO from "libp2p-secio";
import Peer from "libp2p";
import {decode, encode} from "it-length-prefixed";
import pipe from "it-pipe";
import Multiaddr from "multiaddr";
import PeerId from "peer-id";
import * as log from 'loglevel';
import {parseParticle, Particle, stringifyParticle} from "./particle";

export const PROTOCOL_NAME = '/fluence/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class FluenceConnection {

    private readonly selfPeerId: PeerId;
    private node: LibP2p;
    private readonly address: Multiaddr;
    readonly nodePeerId: PeerId;
    private readonly selfPeerIdStr: string;
    private readonly handleCall: (call: Particle) => void;

    constructor(multiaddr: Multiaddr, hostPeerId: PeerId, selfPeerId: PeerId, handleCall: (call: Particle) => void) {
        this.selfPeerId = selfPeerId;
        this.handleCall = handleCall;
        this.selfPeerIdStr = selfPeerId.toB58String();
        this.address = multiaddr;
        this.nodePeerId = hostPeerId;
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
                                let particle = parseParticle(msg);
                                _this.handleCall(particle);
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

    async sendParticle(particle: Particle): Promise<void> {
        this.checkConnectedOrThrow();

        let particleStr = stringifyParticle(particle);
        log.debug("send function call: \n" + JSON.stringify(particle, undefined, 2));

        // create outgoing substream
        const conn = await this.node.dialProtocol(this.address, PROTOCOL_NAME) as {stream: Stream; protocol: string};

        pipe(
            [particleStr],
            // at first, make a message varint
            encode(),
            conn.stream.sink,
        );
    }
}
