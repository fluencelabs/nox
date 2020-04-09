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

import PeerInfo from "peer-info";
import PeerId from "peer-id";
import Peer from "libp2p";
import Websockets from 'libp2p-websockets';
import Mplex from 'libp2p-mplex';
import SECIO from 'libp2p-secio';
import {decode, encode} from 'it-length-prefixed';
import pipe from "it-pipe";
import bs58 from "bs58"
import {
    FunctionCall,
    genUUID,
    makeCallMessage,
    makeFunctionCall, makePeerMsg,
    makeRegisterMessage, makeRelayMsg,
    parseFunctionCall
} from "./function_call";
import {Address} from "./address";
import {privKey1, privKey2} from "./priv_keys";

const PROTOCOL_NAME = '/janus/faas/1.0.0';

export class JanusConnection {
    host: string;
    port: number;
    addr: string;
    nodePeerId: string;
    peerSelf: PeerId;
    node: LibP2p;
    withoutConnection: boolean;
    privateKey: string;
    functions: Map<string, (req: object) => object | void> = new Map();

    constructor(peerId: string, host?: string, port?: number, withoutConnection?: boolean, privateKey?: string) {
        if (!port) { this.port = 9999 } else this.port = port;
        if (!host) { this.host = "127.0.0.1" } else this.host = host;
        this.addr = `/ip4/${this.host}/tcp/${this.port}/ws/p2p/${peerId}`;
        this.nodePeerId = peerId;
        this.withoutConnection = withoutConnection;
        this.privateKey = privateKey;
    }

    /**
     * Sends a message to register the service.
     */
    async registerService(serviceName: string, fn: (req: any) => object | void) {
        let regMsg = makeRegisterMessage(serviceName, PeerId.createFromB58String(this.nodePeerId), this.peerSelf);
        await this.sendFunctionCall(regMsg);

        this.functions.set(serviceName, fn)
    }

    /**
     * Sends a message to unregister the service.
     */
    /*async unregisterService(serviceName: string) {
        if (this.functions.get(serviceName)) {
            let regMsg = makeRegisterMessage(serviceName, PeerId.createFromB58String(this.nodePeerId));
            await this.sendFunctionCall(regMsg);

            this.functions.delete(serviceName)
        }
    }*/

    /**
     * Makes message with response from function.
     */
    private static makeResponseMsg(target: Address, args: any): FunctionCall {
        return makeFunctionCall(genUUID(), target, args, undefined, "response");
    }

    /**
     * Sends remote service call.
     */
    async sendServiceCall(serviceName: string, args: any, name?: string) {
        let regMsg = makeCallMessage(serviceName, args, this.peerSelf, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Sends custom message to the peer.
     */
    async sendPeerMsg(peer: string, msg: any, name?: string) {
        let regMsg = makePeerMsg(PeerId.createFromB58String(peer), msg, this.peerSelf, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Sends custom message to the peer through relay.
     */
    async sendRelayMsg(peer: string, relay: string, msg: any, name?: string) {
        let regMsg = makeRelayMsg(PeerId.createFromB58String(peer), PeerId.createFromB58String(relay), msg, this.peerSelf, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Handle incoming call.
     */
    handleCall(call: FunctionCall): FunctionCall | undefined {
        console.log("FunctionCall received:");
        console.log(JSON.stringify(call, undefined, 2));

        let target = call.target;
        switch (target.type) {
            case "Service":
                let service = this.functions.get(target.service);
                if (service) {
                    try {
                        let result = service(call.arguments);
                        // if result exists, check, that `reply_to` is in request and return response
                        if (typeof result === "object" && call.reply_to) {
                            return JanusConnection.makeResponseMsg(call.reply_to, result)
                        }
                    } catch (e) {
                        return JanusConnection.makeResponseMsg(call.reply_to, { error: `error on execution: ${e}` })
                    }

                } else {
                    console.log(`Unexisted service: ${target.service}`)
                }

                return undefined;
            case "Relay":
                if (target.client === this.peerSelf.toB58String()) {
                    console.log(`relay message: ${call}`);
                } else {
                    console.log(`this relay message is not for me: ${call}`)
                }
                return undefined;
            case "Peer":
                if (target.peer === this.peerSelf.toB58String()) {
                    console.log(`peer message: ${call}`);
                } else {
                    console.log(`this peer message is not for me: ${call}`)
                }
                return undefined;
        }
    }

    /**
     * Establish connection to a node.
     */
    async connect() {

        console.log("start libp2p");

        let peerInfo;

        if (this.privateKey) {
            let peerId = await PeerId.createFromPrivKey(bs58.decode(this.privateKey));
            peerInfo = await PeerInfo.create(peerId);
        } else {
            // generate new private key
            peerInfo = await PeerInfo.create();
            let privKey = peerInfo.id.privKey.bytes;
            let privKeyStr = bs58.encode(privKey);

            console.log(`PRIVATE KEY: ${privKeyStr}`);
        }

        this.peerSelf = peerInfo.id;

        console.log("our peer: ", peerInfo.id.toB58String());

        const node = await Peer.create({
            peerInfo,
            config: {},
            modules: {
                transport: [Websockets],
                streamMuxer: [Mplex],
                connEncryption: [SECIO],
                peerDiscovery: []
            },
        });

        this.node = node;

        if (!this.withoutConnection) {
            await node.start();

            console.log("dialing to the node with address: " + this.addr);

            await node.dial(this.addr);

            let _this = this;

            node.handle([PROTOCOL_NAME], async ({connection, stream}) => {
                pipe(
                    stream.source,
                    decode(),
                    async function (source: AsyncIterable<string>) {
                        for await (const msg of source) {
                            try {
                                let call = parseFunctionCall(msg);
                                let result = _this.handleCall(call);
                                console.log(`call result: ${call}`);
                                if (result) {
                                    await _this.sendFunctionCall(result)
                                }
                            } catch(e) {
                                console.log("error on handling a new incoming message: " + e);
                            }
                        }
                    }
                )
            })
        }
    }

    /**
     * Send FunctionCall to the connected node.
     */
    private async sendFunctionCall(call: FunctionCall) {

        console.log("send function call:");
        console.log(call);

        if (!this.withoutConnection) {
            const conn = await this.node.dialProtocol(this.addr, PROTOCOL_NAME) as {stream: Stream; protocol: string};

            pipe(
                [JSON.stringify(call)],
                // at first, make a message varint
                encode(),
                conn.stream.sink,
            );
        }
    }
}

/**
 * Connects to a janus node.
 * @param peerId in libp2p format. Example:
 *                                      QmUz5ziqFiwuPJnUZehrQ3EyzpHjp22FyQRNH9AxRxKPbp
 *                                      QmcYE4o3HCpotey8Xm87ArERDp9KMgagUnjtKBxuA5vcBY
 * @param host localhost by default
 * @param port 9999 by default
 * @param withoutConnection use Januc client without connection (for debug)
 */
export async function connect(peerId?: string, host?: string, port?: number, withoutConnection?: boolean): Promise<JanusConnection> {

    if (!peerId) peerId = "QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh";
    let connection = new JanusConnection(peerId, host, port, withoutConnection);
    await connection.connect();

    return connection
}


/*
key1: QmcsjjDd8bHFXwAttvyhp7CgaysZhABE2tXFjfPLA5ABJ5
key2: QmRaiyv18eeCcLxfo6aqUpN1xcqqJwxUsxCyhzUE2qgFGJ


/ip4/104.248.25.59/tcp/7001 /ip4/104.248.25.59/tcp/9001/ws QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz
/ip4/104.248.25.59/tcp/7002 /ip4/104.248.25.59/tcp/9002/ws QmVzDnaPYN12QAYLDbGzvMgso7gbRD9FQqRvGZBfeKDSqW
Bootstrap:
/ip4/104.248.25.59/tcp/7770 /ip4/104.248.25.59/tcp/9990/ws QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2

browser1: con = await connectWithKey(1, "QmX6yYZd4iLW7YpmZz4waLrtb5Y9f5v3PPGEmNGh9k3iW2", "104.248.25.59", 9990)
browser2: con = await connectWithKey(2, "QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "104.248.25.59", 9001)
browser1: con.sendRelayMsg("QmRaiyv18eeCcLxfo6aqUpN1xcqqJwxUsxCyhzUE2qgFGJ", "QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "some relay msg", "some name")
browser1: con.sendPeerMsg("QmRaiyv18eeCcLxfo6aqUpN1xcqqJwxUsxCyhzUE2qgFGJ", {msg: "peer msg", random_field: 123}, "some name")
 */
export async function connectWithKey(keyNumber: number, peerId?: string, host?: string, port?: number): Promise<JanusConnection> {
    if (!peerId) peerId = "QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh";

    let key;
    if (keyNumber === 1) {
        key = privKey1;
    } else {
        key = privKey2;
    }

    let connection = new JanusConnection(peerId, host, port, false, key);
    await connection.connect();

    return connection
}

if (typeof window !== 'undefined') {
    (<any>window).connect = connect;
    (<any>window).connectWithKey = connectWithKey;
}

