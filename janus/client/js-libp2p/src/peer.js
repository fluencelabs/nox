'use strict';

import Peer from 'libp2p';
import Websockets from 'libp2p-websockets';
import Mplex from 'libp2p-mplex';
import SECIO from 'libp2p-secio';
import PeerInfo from 'peer-info';
import pipe from 'it-pipe';
import lp from 'it-length-prefixed';
import peer_id from 'peer-id';

const protocol_name = '/janus/peer/1.0.0';

async function create_peer() {
    const peer_info = await PeerInfo.create();
    //peer_info.multiaddrs.add('/ip4/127.0.0.1/tcp/5555');
    const node = await Peer.create({
        peer_info,
        modules: {
            transport: [Websockets],
            streamMuxer: [Mplex],
            connEncryption: [SECIO],
        }
    });

    await node.start();
    return node
}

function make_relay_message(input) {
    const parsed_input = JSON.parse(input);
    const dst_peer_id = peer_id.createFromB58String(parsed_input.dst);
    return JSON.stringify({
        action: "Relay",
        // TODO: maybe there is an effective way of getting the internal data field of Buffer
        dst_id: dst_peer_id.toBytes().toJSON().data,
        data: parsed_input.message.split('').map (function (c) { return c.charCodeAt (0); })
    });
}

function bin2String(array) {
    var result = "";
    for (var i = 0; i < array.length; i++) {
        result += String.fromCharCode(array[i]);
    }
    return result;
}

async function send_message(peer, node_addr, message, log) {
    var relay_message;
    try {
        relay_message = make_relay_message(message);
    } catch (e) {
        log("incorrect string provided");
        return;
    }

    // need to dial each time - there is the OneShotHandler handler on the rust side
    const { stream } = await peer.dialProtocol(node_addr, protocol_name);
    log("relaying " + relay_message);

    pipe(
        [relay_message],
        // at first, make a message varint
        lp.encode(),
        stream.sink,
    )
}

// Does following actions:
//  - starts the libp2p
//  - connects to the provided node_addr
//  - registers handler that output incoming messages by given print_incoming
//  - outputs some logs by provided print_log
export async function start_peer(node_addr, print_log, print_incoming) {
    print_log('starting libp2p');
    const [peer] = await Promise.all([create_peer()]);

    print_log("dialing to the node by " + node_addr);
    await peer.dial(node_addr);

    // adjust the listener
    await peer.handle(protocol_name, ({ stream }) => {
        pipe(
            stream.source,
            lp.decode(),
            async function (source) {
                for await (const msg of source) {
                    try {
                        let msg_decoded = JSON.parse(msg);
                        let src_peer_id = peer_id.createFromBytes(Buffer.from(msg_decoded.src_id));
                        print_incoming(src_peer_id.toB58String() + ": \"" + bin2String(msg_decoded.data) + "\"");
                    } catch(e) {
                        print_incoming("error on handling a new incoming message: " + e);
                    }
                }
            }
        )
    });

    print_log('libp2p started!\nExample of a relay message {\"dst\": \"' + peer.peerInfo.id.toB58String() + '\", \"message\": \"hello\"}');

    // return the function that could be used for sending messages
    return async function(message) {
        await send_message(peer, node_addr, message, print_log);
    };
}
