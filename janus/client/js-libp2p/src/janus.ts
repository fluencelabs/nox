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

import {Address, createRelayAddress} from "./address";
import {
    FunctionCall,
    genUUID,
    makeCall,
    makeFunctionCall,
    makePeerCall, makeRegisterMessage,
    makeRelayCall,
    parseFunctionCall
} from "./function_call";
import * as PeerId from "peer-id";
import {Services} from "./services";
import {Subscriptions} from "./subscriptions";
import bs58 from "bs58";
import * as PeerInfo from "peer-info";
import Websockets from "libp2p-websockets";
import Mplex from "libp2p-mplex";
import SECIO from "libp2p-secio";
import Peer from "libp2p";
import {decode, encode} from "it-length-prefixed";
import pipe from "it-pipe";

export const PROTOCOL_NAME = '/janus/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class JanusClient {
    private readonly host: string;
    private readonly port: number;
    private readonly nodePeerId: PeerId;
    private readonly nodePeerIdStr: string;
    private readonly selfPeerInfo: PeerInfo;
    readonly selfPeerIdStr: string;
    private readonly node: LibP2p;
    readonly replyToAddress: Address;
    private readonly address: string;

    // connection status. If `Disconnected`, it cannot be reconnected
    private status: Status = Status.Initializing;

    private services: Services = new Services();

    private subscriptions: Subscriptions = new Subscriptions();

    private constructor(host: string, port: number, selfPeerInfo: PeerInfo, node: LibP2p, nodePeerId: PeerId) {
        this.host = host;
        this.port = port;
        this.selfPeerInfo = selfPeerInfo;
        this.selfPeerIdStr = selfPeerInfo.id.toB58String();
        this.nodePeerId = nodePeerId;
        this.nodePeerIdStr = nodePeerId.toB58String();
        this.replyToAddress = createRelayAddress(this.nodePeerIdStr, this.selfPeerIdStr);
        this.node = node;
        this.address = `/ip4/${this.host}/tcp/${this.port}/ws/p2p/${this.nodePeerIdStr}`;
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
                                let call = parseFunctionCall(msg);
                                let response = _this.handleCall(call);

                                // send a response if it exists, do nothing otherwise
                                if (response) {
                                    await _this.sendFunctionCall(response);
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
            throw `can't start receiving. Status: ${this.status}`;
        }
    }

    /**
     * Makes message with response from function. Without reply_to field.
     */
    private static responseCall(target: Address, args: any): FunctionCall {
        return makeFunctionCall(genUUID(), target, args, undefined, "response");
    }

    /**
     * Sends remote service_id call.
     */
    async sendServiceCall(serviceName: string, args: any, name?: string) {
        let regMsg = makeCall(serviceName, args, this.replyToAddress, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Sends custom message to the peer.
     */
    async sendPeerCall(peer: string, msg: any, name?: string) {
        let regMsg = makePeerCall(PeerId.createFromB58String(peer), msg, this.replyToAddress, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Sends custom message to the peer through relay.
     */
    async sendRelayCall(peer: string, relay: string, msg: any, name?: string) {
        let regMsg = makeRelayCall(PeerId.createFromB58String(peer), PeerId.createFromB58String(relay), msg, this.replyToAddress, name);
        await this.sendFunctionCall(regMsg);
    }

    /**
     * Send FunctionCall to the connected node.
     */
    async sendFunctionCall(call: FunctionCall) {
        this.checkConnection();

        console.log("send function call:");
        console.log(call);

        // create outgoing substream
        const conn = await this.node.dialProtocol(this.address, PROTOCOL_NAME) as {stream: Stream; protocol: string};

        pipe(
            [JSON.stringify(call)],
            // at first, make a message varint
            encode(),
            conn.stream.sink,
        );
    }

    /**
     * Handle incoming call.
     * If FunctionCall returns - we should send it as a response.
     */
    handleCall(call: FunctionCall): FunctionCall | undefined {
        console.log("FunctionCall received:");
        console.log(call);

        // if other side return an error - handle it
        // TODO do it in the protocol
        /*if (call.arguments.error) {
            this.handleError(call);
        } else {

        }*/

        let target = call.target;
        // TODO switch to multiaddress
        switch (target.type) {
            case "Service":
                try {
                    // call of the service, service should handle response sending, error handling, requests to other services
                    let applied = this.services.applyToService(target.service_id, call);

                    // if the request hasn't been applied, there is no such service. Return an error.
                    if (!applied) {
                        console.log(`there is no service ${target.service_id}`);
                        return JanusClient.responseCall(call.reply_to, { reason: `there is no such service`, msg: call });
                    }
                } catch (e) {
                    // if service throw an error, return it to the sender
                    return JanusClient.responseCall(call.reply_to, { reason: `error on execution: ${e}`, msg: call });
                }

                return undefined;
            case "Relay":
                if (target.client === this.selfPeerIdStr) {
                    console.log(`relay message: ${call}`);
                    this.subscriptions.applyToSubscriptions(call)
                } else {
                    console.log(`this relay message is not for me: ${call}`)
                }
                return undefined;
            case "Peer":
                if (target.peer === this.selfPeerIdStr) {
                    console.log(`peer message: ${call}`);
                    this.subscriptions.applyToSubscriptions(call)
                } else {
                    console.log(`this peer message is not for me: ${call}`)
                }
                return undefined;
        }
    }

    checkConnection() {
        if (this.status !== Status.Connected) {
            throw `connection is in ${this.status} state`
        }
    }

    /**
     * Sends a message to register the service_id.
     */
    async registerService(serviceName: string, fn: (req: FunctionCall) => void) {
        this.checkConnection();
        let regMsg = makeRegisterMessage(serviceName, this.nodePeerId, this.selfPeerInfo.id);
        await this.sendFunctionCall(regMsg);

        this.services.addService(serviceName, fn, regMsg)
    }

    // subscribe new hook for every incoming call, to handle in-service responses and other different cases
    // the hook will be deleted if it will return `true`
    subscribe(f: (call: FunctionCall) => (boolean | undefined)) {
        this.checkConnection();
        this.subscriptions.subscribe(f)
    }

    /**
     * Sends a message to unregister the service_id.
     */
    async unregisterService(serviceName: string) {
        if (this.services.deleteService(serviceName)) {
            // TODO unregister in fluence network when it will be supported
            // let regMsg = makeRegisterMessage(serviceName, PeerId.createFromB58String(this.nodePeerId));
            // await this.sendFunctionCall(regMsg);
        }
    }

    async disconnect() {
        await this.node.stop();
        this.status = Status.Disconnected;
    }

    static async connect(nodePeerId: string, host?: string, port?: number, privateKey?: string) {

        if (!host) {
            host = "localhost"
        }

        if (!port) {
            port = 9999
        }

        let peerInfo;
        if (privateKey) {
            let peerId = await PeerId.createFromPrivKey(bs58.decode(privateKey));
            peerInfo = await PeerInfo.create(peerId);
        } else {
            // generate new private key
            peerInfo = await PeerInfo.create();
        }

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

        let connection = new JanusClient(host, port, peerInfo, node, PeerId.createFromB58String(nodePeerId));

        await connection.startReceiving();

        return connection;
    }
}

/**
 * Connects to a janus node.
 * @param peerId in libp2p format. Example:
 *                                      QmUz5ziqFiwuPJnUZehrQ3EyzpHjp22FyQRNH9AxRxKPbp
 *                                      QmcYE4o3HCpotey8Xm87ArERDp9KMgagUnjtKBxuA5vcBY
 * @param host localhost by default
 * @param port 9999 by default
 * @param privateKey in base58, will create new private key and peerId if empty
 */
export async function connect(peerId: string, host?: string, port?: number, privateKey?: string): Promise<JanusClient> {
    return await JanusClient.connect(peerId, host, port, privateKey)
}

