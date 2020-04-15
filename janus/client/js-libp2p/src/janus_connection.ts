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
    makeCall,
    makeFunctionCall, makePeerCall,
    makeRegisterMessage, makeRelayCall,
    parseFunctionCall
} from "./function_call";
import {Address, createRelayAddress} from "./address";
import {keys, privKey1, privKey2} from "./priv_keys";
import {Services} from "./services";
import {Subscriptions} from "./subscriptions";

const PROTOCOL_NAME = '/janus/faas/1.0.0';

export class JanusConnection {
    readonly host: string;
    readonly port: number;
    readonly addr: string;
    readonly nodePeerId: string;
    peerSelf: PeerId;
    private node: LibP2p;
    readonly hasConnection: boolean;
    private privateKey: string;
    replyToAddress: Address;

    private services: Services = new Services();

    private subscriptions: Subscriptions = new Subscriptions();

    constructor(peerId: string, host?: string, port?: number, hasConnection?: boolean, privateKey?: string) {
        if (!port) { this.port = 9999 } else this.port = port;
        if (!host) { this.host = "127.0.0.1" } else this.host = host;
        this.addr = `/ip4/${this.host}/tcp/${this.port}/ws/p2p/${peerId}`;
        this.nodePeerId = peerId;
        if (hasConnection) {
            this.hasConnection = hasConnection;
        } else {
            this.hasConnection = true;
        }

        this.privateKey = privateKey;

        // infinite subscription that logs all peer and relay calls
        this.subscriptions.subscribe(req => {
            console.log("processing in subscription");
            console.log(req);
            return undefined;
        })
    }

    /**
     * Sends a message to register the service_id.
     */
    async registerService(serviceName: string, fn: (req: any) => void) {
        let regMsg = makeRegisterMessage(serviceName, PeerId.createFromB58String(this.nodePeerId), this.peerSelf);
        await this.sendFunctionCall(regMsg);

        this.services.addService(serviceName, fn)
    }


    subscribe(f: (call: FunctionCall) => (boolean | undefined)) {
        this.subscriptions.subscribe(f)
    }

    /**
     * Sends a message to unregister the service_id.
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
     * Handle incoming call.
     */
    handleCall(call: FunctionCall): FunctionCall | undefined {
        console.log("FunctionCall received:");
        console.log(call);

        let target = call.target;
        switch (target.type) {
            case "Service":
                try {
                    // call of the service, service should handle response sending, error handling, requests to other services
                    let applied = this.services.applyToService(target.service_id, call);
                    if (!applied) {
                        console.log(`there is no service ${target.service_id}`)
                    }
                } catch (e) {
                    return JanusConnection.responseCall(call.reply_to, { error: `error on execution: ${e}` })
                }

                return undefined;
            case "Relay":
                if (target.client === this.peerSelf.toB58String()) {
                    console.log(`relay message: ${call}`);
                    this.subscriptions.applyToSubscriptions(call)
                } else {
                    console.log(`this relay message is not for me: ${call}`)
                }
                return undefined;
            case "Peer":
                if (target.peer === this.peerSelf.toB58String()) {
                    console.log(`peer message: ${call}`);
                    this.subscriptions.applyToSubscriptions(call)
                } else {
                    console.log(`this peer message is not for me: ${call}`)
                }
                return undefined;
        }
    }

    async disconnect() {
        await this.node.stop();
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

        this.replyToAddress = createRelayAddress(this.nodePeerId, this.peerSelf.toB58String());

        if (this.hasConnection) {
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
                                _this.handleCall(call);
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
    async sendFunctionCall(call: FunctionCall) {

        console.log("send function call:");
        console.log(call);

        if (this.hasConnection) {
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
 * @param hasConnection possible to use Janus client without connection (for debug)
 */
export async function connect(peerId?: string, host?: string, port?: number, hasConnection?: boolean): Promise<JanusConnection> {

    if (!peerId) peerId = "QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh";
    let connection = new JanusConnection(peerId, host, port, hasConnection);
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
browser1: con.sendRelayCall("QmRaiyv18eeCcLxfo6aqUpN1xcqqJwxUsxCyhzUE2qgFGJ", "QmVL33cyaaGLWHkw5ZwC7WFiq1QATHrBsuJeZ2Zky7nDpz", "some relay msg", "some name")
browser1: con.sendPeerCall("QmRaiyv18eeCcLxfo6aqUpN1xcqqJwxUsxCyhzUE2qgFGJ", {msg: "peer msg", random_field: 123}, "some name")
 */
export async function connectWithKey(keyNumber: number, peerId?: string, host?: string, port?: number): Promise<JanusConnection> {
    if (!peerId) peerId = "QmWySxQsFWPHdTLMqhJb4DYrTiFEge2tLe7FksRGHuPiTh";

    let key = keys.get(keyNumber) as string;

    let connection = new JanusConnection(peerId, host, port, false, key);
    await connection.connect();

    return connection
}
