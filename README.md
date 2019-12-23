# arqada
Pluggable relay infrastructure for p2p-enabled applications. Compatible with YJS, automerge, sync9, etc.

# Run
## Rust
```bash
RUST_LOG="libp2p_secio=trace" cargo run
```

It will print something like
```
peer id: QmaVSMEXP5w65aebS5STT21S6WZPeeP29VHkG4E542ZJFf
Listening on /ip4/127.0.0.1/tcp/30000/p2p/QmaVSMEXP5w65aebS5STT21S6WZPeeP29VHkG4E542ZJFf
```


## JS
```bash
cd js
DEBUG="*" node run.js /ip4/127.0.0.1/tcp/30000/p2p/WHATEVER_KEY_IS_PRINTED_BY_RUST
```
It will fail like this:
```
  libp2p:secio 1.1 identify +0ms
  libp2p:conn:out:QmYFby86 closing connection to QmaVSMEXP5w65aebS5STT21S6WZPeeP29VHkG4E542ZJFf +10ms
  libp2p:error could not connect to discovered peer Error: dialed to the wrong peer, Ids do not match
    at PeerId.createFromPubKey (/Users/folex/Development/arqada/js/node_modules/libp2p-secio/src/handshake/crypto.js:80:25)
    at computeDigest (/Users/folex/Development/arqada/js/node_modules/peer-id/src/index.js:163:7)
    at computeDigest (/Users/folex/Development/arqada/js/node_modules/peer-id/src/index.js:150:5)
    at computePeerId (/Users/folex/Development/arqada/js/node_modules/peer-id/src/index.js:159:3)
    at Function.exports.createFromPubKey (/Users/folex/Development/arqada/js/node_modules/peer-id/src/index.js:220:3)
    at Object.exports.identify (/Users/folex/Development/arqada/js/node_modules/libp2p-secio/src/handshake/crypto.js:72:10)
    at waterfall (/Users/folex/Development/arqada/js/node_modules/libp2p-secio/src/handshake/propose.js:24:14)
    at nextTask (/Users/folex/Development/arqada/js/node_modules/libp2p-secio/node_modules/async/waterfall.js:16:14)
    at next (/Users/folex/Development/arqada/js/node_modules/libp2p-secio/node_modules/async/waterfall.js:23:9)
    at /Users/folex/Development/arqada/js/node_modules/libp2p-secio/node_modules/async/internal/onlyOnce.js:12:16 +0ms

Error: {"code":"DIAL_ABORTED"}
/Users/folex/Development/arqada/js/run.js:71
    throw err
    ^

Error: Dial was aborted
    at createError (/Users/folex/Development/arqada/js/node_modules/err-code/index.js:4:44)
    at DIAL_ABORTED (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/errors.js:7:23)
    at Queue.abort (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/dialer/queue.js:144:21)
    at Queue.denylist (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/dialer/queue.js:169:10)
    at ClassIsWrapper.connectionFSM.once (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/dialer/queue.js:252:12)
    at Object.onceWrapper (events.js:286:20)
    at ClassIsWrapper.emit (events.js:198:13)
    at ClassIsWrapper.emit (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/connection/base.js:36:13)
    at ClassIsWrapper.close (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/connection/base.js:27:12)
    at encryptedConn.switch.crypto.encrypt (/Users/folex/Development/arqada/js/node_modules/libp2p/src/switch/connection/index.js:324:25)
```