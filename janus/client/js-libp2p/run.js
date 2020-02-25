'use strict'

const Libp2p = require('libp2p')
const TCP = require('libp2p-tcp')
const Mplex = require('libp2p-mplex')
const SECIO = require('libp2p-secio')
const PeerInfo = require('peer-info')
const pipe = require('it-pipe')
const lp = require('it-length-prefixed')
const peer_id = require('peer-id')

const protocol_name = '/janus/peer/1.0.0';

const createNode = async () => {
    const peer_info = await PeerInfo.create();
    //peer_info.multiaddrs.add('/ip4/127.0.0.1/tcp/5555');
    const node = await Libp2p.create({
        peer_info,
        modules: {
            transport: [TCP],
            streamMuxer: [Mplex],
            connEncryption: [SECIO],
        }
    });

    await node.start();
    return [node, peer_info]
};

var NODE_ADDR = "/ip4/127.0.0.1/tcp/9999";
var NODE_PEER_ID = "";
if (process.argv.length > 2) {
    NODE_ADDR = process.argv[2];
    NODE_PEER_ID = process.argv[3]
}

NODE_ADDR = NODE_ADDR + "/p2p/" + NODE_PEER_ID;
console.log("connecting to ", NODE_ADDR);

function make_relay_message(input) {
    const parsed_input = JSON.parse(input);
    const dst_peer_id = peer_id.createFromB58String(parsed_input.dst);
    return JSON.stringify({
        action: "Relay",
        // TODO: maybe not so effective way of getting the internal data field of Buffer
        dst_id: dst_peer_id.toBytes().toJSON().data,
        data: parsed_input.message.split('').map (function (c) { return c.charCodeAt (0); })
    });
}

;(async () => {
    const [ [node, peer_info] ] = await Promise.all([createNode()]);

    console.log("dialing to the node");
    await node.dial(NODE_ADDR);

    // TODO: id for peer_info doesn't match the one from the rust side
    console.log("example of a relay message {\"dst\": \"" + peer_info.id.toB58String() + "\", \"message\": \"hello\"}");

    // adjust the listener
    await node.handle(protocol_name, ({ stream }) => {
        pipe(
            stream.source,
            lp.decode(),
            async function (source) {
                for await (const msg of source) {
                    try {
                        let msg_decoded = JSON.parse(msg);
                        let src_peer_id = peer_id.createFromBytes(Buffer.from(msg_decoded.src_id));
                        console.log(src_peer_id.toB58String() + ": " + msg_decoded.data);
                    } catch(e) {
                        console.log("error on handling a new incoming message: " + e);
                    }
                }
            }
        )
    });

    process.stdin.on("data", async function (event) {
        var relay_message;
        try {
            relay_message = make_relay_message(event);
        } catch (e) {
            console.log("incorrect string provided");
            return;
        }

        // need to dial each time - there is the OneShotHandler handler on the rust side
        const { stream } = await node.dialProtocol(NODE_ADDR, protocol_name)
        console.log("relaying " + relay_message);

        pipe(
            [relay_message],
            // at first, make a message varint
            lp.encode(),
            stream.sink,
        )
    });
})();
