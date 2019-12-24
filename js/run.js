'use strict'

const libp2p = require('libp2p')
const TCP = require('libp2p-tcp')
const Mplex = require('libp2p-mplex')
const SECIO = require('libp2p-secio')
const PeerInfo = require('peer-info')
const FloodSub = require('libp2p-floodsub')
const CID = require('cids')
const KadDHT = require('libp2p-kad-dht')
const defaultsDeep = require('@nodeutils/defaults-deep')
const waterfall = require('async/waterfall')
const parallel = require('async/parallel')
const readline = require('readline');
const Swarm = require('libp2p-switch')
const once = require('once');

// TODO WHY: In Rust it's QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw, in JS it becomes 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G
var RUST_PEER = "/ip4/127.0.0.1/tcp/30000/p2p/12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G";

if (process.argv.length > 2) {
    RUST_PEER = process.argv[2];
    console.log("Using peer from argument: ", RUST_PEER);
}

class MyBundle extends libp2p {
  constructor(_options) {
      const defaults = {
          modules: {
              transport: [TCP], // TODO: try udp?
              streamMuxer: [Mplex],
              connEncryption: [SECIO],
              pubsub: FloodSub
          },
          config: {
              pubsub: {
                  enabled: true,
                  emitSelf: false,
                  signMessages: false, // Rust doesn't sign floodsub messages
                  strictSigning: false // Rust doesn't sign floodsub messages
              },
              relay: { // Rust doesn't support relay
                  enabled: false,
              }
          }
      };

    super(defaultsDeep(_options, defaults))
  }
}

function createNode(callback) {
    let node;

    waterfall([
        (cb) => {
            cb = once(cb);
            PeerInfo.create().then((pi) => cb(null, pi)).catch((err) => cb(err))
        },
        (peerInfo, cb) => {
            console.log("Local peer created " + peerInfo.id.toB58String());
            peerInfo.multiaddrs.add('/ip4/127.0.0.1/tcp/0');
            node = new MyBundle({
                peerInfo
            });
            node.on('peer:discovery', (peer) => {
                console.log('Discovered peer:', peer.id.toB58String())
                // node.dial(peer, () => { })
            });
            node.on('peer:connect', (peer) => {
                console.log('Connection established to:', peer.id.toB58String())
            });
            node.on('connection:start', (peerInfo) => {
                console.log('Connection started to:', peerInfo.id.toB58String())
            });
            node.on('connection:end', (peerInfo) => {
                console.log('Connection ended with:', peerInfo.id.toB58String())
            });
            node.on('error', (err) => {
                console.error('Node received error:', err);
            });
            node.start(cb);
        },
        (cb) => {
            console.log("node started");
            console.log("will dial " + RUST_PEER);
            node.dial(RUST_PEER, cb)
        },
        (cb) => {
            console.log("node dialed");
            node.pubsub.subscribe("5zKTH5FR", (msg) => {
                console.log("floodsub received", msg.data.toString(), 'from', msg.from)
            }, {}, cb);
        },
        (cb) => {
            console.log('floodsub subscribed');
            cb()
        }
    ], (err) => callback(err, node))
}

createNode((err) => {
  if (err) {
    console.log('\nError:', JSON.stringify(err));
    throw err
  }
});
