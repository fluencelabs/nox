import {createRelayAddress, Address} from "./address";
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

export const PROTOCOL_NAME = '/janus/faas/1.0.0';

enum Status {
    Initializing = "Initializing",
    Connected = "Connected",
    Disconnected = "Disconnected"
}

export class JanusConnection {

    private readonly host: string;
    private readonly port: number;
    private readonly selfPeerInfo: PeerInfo;
    readonly replyToAddress: Address;
    private node: LibP2p;
    private readonly address: string;
    private readonly nodePeerId: PeerId;
    private readonly selfPeerId: string;
    private readonly handleCall: (call: FunctionCall) => FunctionCall | undefined;

    constructor(host: string, port: number, hostPeerId: PeerId, selfPeerInfo: PeerInfo, handleCall: (call: FunctionCall) => FunctionCall | undefined) {
        this.selfPeerInfo = selfPeerInfo;
        this.host = host;
        this.port = port;
        this.handleCall = handleCall;
        this.selfPeerId = selfPeerInfo.id.toB58String();
        this.address = `/ip4/${host}/tcp/${port}/ws/p2p/${hostPeerId}`;
        this.nodePeerId = hostPeerId;
        this.replyToAddress = createRelayAddress(hostPeerId.toB58String(), this.selfPeerId);
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
        let regMsg = makeRelayCall(PeerId.createFromB58String(peer), PeerId.createFromB58String(relay), msg, this.replyToAddress, name);
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

    checkConnectedOrThrow() {
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
        let regMsg = makeRegisterMessage(serviceId, this.nodePeerId, this.selfPeerInfo.id);
        await this.sendCall(regMsg);
    }
}