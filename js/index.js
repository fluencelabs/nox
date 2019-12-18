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


class MyBundle extends libp2p {
  constructor(_options) {
    const defaults = {
      modules: {
        transport: [TCP],
        streamMuxer: [Mplex],
        connEncryption: [], // TODO: [SECIO]
      },
    };

    super(defaultsDeep(_options, defaults))
  }
}

function createNode(callback) {
  let node;

  waterfall([
    (cb) => PeerInfo.create(cb),
    (peerInfo, cb) => {
      console.log("Peer created " + JSON.stringify(peerInfo.id));
      peerInfo.multiaddrs.add('/ip4/127.0.0.1/tcp/0');
      node = new MyBundle({
        peerInfo
      });
      node.start(cb);
    },
    (r, cb) => {
      console.log("node started " + JSON.stringify(r));
      node.dial("/ipv4/127.0.0.1/tcp/30000", cb)
    },
    (conn, cb) => {
      console.log("connected " + JSON.stringify(conn));
    }
  ], (err) => callback(err, node))
}

createNode((err) => console.log("error " + JSON.stringify(err)));

// let fsub;
// let node;
// const bootstrapAddr = process.argv[2];

// waterfall([
//   (cb) => createNode(cb),
//   (node_, cb) => {
//     node = node_
//     console.log("My ID:  " + node.peerInfo.id._idB58String)
//     fsub = new FloodSub(node)
//     fsub.start(cb)
//   },
//   (cb) => {
//     fsub.on(topicName, (data) => {
//       const peerIdStr = data.from
//       const peerIdTruncdStr = peerIdStr.substr(0,2) + "*" + peerIdStr.substr(peerIdStr.length-6,6)
//       const messageStr = data.data
//       console.log("<peer " + peerIdTruncdStr + ">: " + messageStr)
//     })
//     fsub.subscribe(topicName)
//
//     node.dial(bootstrapAddr, cb)
//   },
// ], (err) => {
//   if (err) {
//     console.log('Error:', err)
//     throw err
//   }
//
//   console.log("Connected to: ", bootstrapAddr)
//
//   var rl = readline.createInterface(process.stdin, process.stdout);
//   rl.setPrompt('');
//   rl.prompt();
//   rl.on('line', function(line) {
//         fsub.publish(topicName, line)
//         rl.prompt();
//   }).on('close',function(){
//         process.exit(0);
//   })
//
// })