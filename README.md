# arqada
Pluggable relay infrastructure for p2p-enabled applications. Compatible with YJS, automerge, sync9, etc.

## Rust
```bash
RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info" cargo run
```

It will print something like
```
peer id: QmaVSMEXP5w65aebS5STT21S6WZPeeP29VHkG4E542ZJFf
Listening on /ip4/127.0.0.1/tcp/30000/p2p/QmaVSMEXP5w65aebS5STT21S6WZPeeP29VHkG4E542ZJFf
```


## JS
```bash
cd js
DEBUG="*,-latency-monitor:LatencyMonitor" node run.js /ip4/127.0.0.1/tcp/30000/p2p/WHATEVER_KEY_IS_PRINTED_BY_RUST
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

## Digging
- Managed to patch JS, so the error gives more info:
```
  libp2p:error could not connect to discovered peer Error: dialed to the wrong peer, Ids do not match. expected: 12D3KooWFha52q5MRwbCAcQUef6Vn4Up83bccLqnfqPa2KQtP9eU actual: 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA
```
- JS sees public key as `CAESIBoAyUXv/1nd+R5WjJz9mF0tUPGGDfonzCoKdYNB726V`, Rust as `GgDJRe//Wd35HlaMnP2YXS1Q8YYN+ifMKgp1g0HvbpU`. Keys are the same, both in base64, JS prefixed it with some info.
  - `echo CAESIBoAyUXv/1nd+R5WjJz9mF0tUPGGDfonzCoKdYNB726V | base64 -D | xxd` will give you a hex dump
- If connect to `12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA`, JS and Rust will both hang
  - Rust
    ```
    [2019-12-23T14:08:38Z TRACE libp2p_tcp] Incoming connection from /ip4/127.0.0.1/tcp/64966 at /ip4/127.0.0.1/tcp/30000
something happened in swarm runloop
[2019-12-23T14:08:38Z TRACE multistream_select::protocol] Received message: Header(V1)
[2019-12-23T14:08:38Z TRACE multistream_select::protocol] Received message: Protocol(Protocol(b"/secio/1.0.0"))
[2019-12-23T14:08:38Z DEBUG multistream_select::listener_select] Listener: confirming protocol: /secio/1.0.0
[2019-12-23T14:08:38Z DEBUG multistream_select::listener_select] Listener: sent confirmed protocol: /secio/1.0.0
[2019-12-23T14:08:38Z DEBUG libp2p_secio] Starting secio upgrade
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] agreements proposition: P-256,P-384
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] ciphers proposition: AES-128,AES-256,TwofishCTR
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] digests proposition: SHA256,SHA512
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] starting handshake; local nonce = [136, 165, 206, 114, 252, 162, 96, 241, 142, 101, 38, 46, 11, 56, 212, 0]
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] sending proposition to remote
[2019-12-23T14:08:38Z TRACE tokio_io::framed_read] attempting to decode a frame
[2019-12-23T14:08:38Z TRACE tokio_io::framed_read] frame decoded from buffer
[2019-12-23T14:08:38Z DEBUG libp2p_secio::handshake] selected cipher: Aes128
[2019-12-23T14:08:38Z DEBUG libp2p_secio::handshake] selected hash: Sha256
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] received proposition from remote; pubkey = Rsa(PublicKey([48, 130, 1, 10, 2, 130, 1, 1, 0, 227, 178, 39, 30, 85, 158, 92, 224, 31, 70, 229, 204, 254, 175, 240, 119, 104, 64, 62, 54, 115, 54, 104, 50, 34, 54, 95, 184, 233, 98, 100, 65, 50, 4, 111, 134, 39, 68, 221, 121, 166, 154, 145, 137, 166, 33, 224, 86, 71, 247, 1, 76, 82, 149, 22, 162, 87, 173, 177, 74, 113, 14, 32, 0, 55, 12, 104, 74, 188, 72, 210, 144, 71, 179, 48, 54, 93, 125, 90, 194, 25, 26, 110, 12, 51, 36, 160, 245, 70, 146, 62, 40, 86, 139, 126, 145, 35, 201, 99, 128, 235, 245, 220, 114, 25, 132, 80, 125, 140, 131, 64, 174, 119, 165, 133, 218, 191, 126, 43, 70, 22, 94, 106, 95, 168, 205, 60, 135, 65, 42, 107, 51, 120, 98, 21, 74, 96, 65, 102, 243, 80, 194, 83, 222, 21, 99, 38, 177, 90, 236, 155, 80, 92, 246, 182, 69, 181, 45, 153, 126, 239, 123, 7, 148, 219, 226, 198, 198, 37, 68, 192, 135, 38, 33, 232, 24, 192, 127, 69, 226, 117, 44, 213, 255, 164, 87, 141, 225, 184, 215, 173, 219, 22, 101, 109, 2, 247, 226, 214, 187, 53, 46, 105, 86, 125, 191, 8, 182, 116, 22, 243, 56, 236, 183, 156, 148, 88, 12, 170, 210, 105, 117, 197, 98, 62, 75, 230, 196, 74, 201, 253, 237, 130, 140, 28, 229, 58, 8, 75, 95, 88, 154, 157, 130, 17, 174, 70, 193, 174, 202, 1, 199, 47, 253, 77, 85, 2, 3, 1, 0, 1])); nonce = [207, 254, 158, 5, 72, 239, 44, 103, 188, 13, 111, 183, 208, 3, 67, 166]
[2019-12-23T14:08:38Z TRACE libp2p_secio::handshake] sending exchange to remote
[2019-12-23T14:08:38Z TRACE tokio_io::framed_read] attempting to decode a frame
    ```
  - JS
    ```
    Peer created QmTQqEj3VFUqyxkQCcRYLixH3Y1qhYKNCrgUBW7HCsVoMp
  libp2p:switch:dialer create: 8 peer limit, 30000 dial timeout +0ms
  libp2p:connection-manager options: {"maxPeers":null,"minPeers":25,"maxData":null,"maxSentData":null,"maxReceivedData":null,"maxEventLoopDelay":null,"pollInterval":2000,"movingAverageInterval":60000,"defaultPeerValue":1,"maxPeersPerProtocol":{}} +0ms
  libp2p:switch:transport adding Circuit +0ms
  libp2p libp2p is starting +0ms
  libp2p:switch:transport adding TCP +3ms
  libp2p:switch The switch is starting +0ms
  libp2p:tcp:listen Listening on 0 127.0.0.1 +0ms
  libp2p:switch The switch has started +18ms
  libp2p libp2p has started +23ms
node started
will dial /ip4/127.0.0.1/tcp/30000/p2p/12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA
Discovered: 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA
  libp2p connecting to discovered peer +3ms
  libp2p:switch:dial starting dial queue to 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +0ms
  libp2p:connection-manager QmTQqEj3VFUqyxkQCcRYLixH3Y1qhYKNCrgUBW7HCsVoMp: connected to 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +32ms
  libp2p:connection-manager checking limit of maxPeers. current value: 1 of Infinity +0ms
  libp2p:connection-manager checking protocol limit. current value of ip4 is 1 +0ms
  libp2p:connection-manager checking protocol limit. current value of tcp is 1 +0ms
  libp2p:connection-manager checking protocol limit. current value of ipfs is 1 +0ms
  libp2p:conn:out:QmTQqEj3 dialing 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +0ms
  libp2p:conn:out:QmTQqEj3 dialing transport TCP +0ms
  libp2p:switch:transport dialing TCP [ '/ip4/127.0.0.1/tcp/30000/ipfs/12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA' ] +31ms
  libp2p:switch:dialer dialMany:start +40ms
  libp2p:switch:dialer dialSingle: 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA:/ip4/127.0.0.1/tcp/30000/ipfs/12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +1ms
  libp2p:switch:dialer:queue work:start +0ms
  libp2p:tcp:dial Connecting to 30000 127.0.0.1 +0ms
  libp2p:switch:dialer:queue work:success +3ms
  libp2p:switch:dialer dialMany:success +4ms
  libp2p:conn:out:QmTQqEj3 successfully dialed 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +10ms
  mss:dialer	 (f7cgva) dialer handle conn +0ms
  mss:dialer	 (f7cgva) writing multicodec: /multistream/1.0.0 +1ms
  mss:dialer	 (f7cgva) received ack: /multistream/1.0.0 +5ms
  mss:dialer	 (f7cgva) handshake success +1ms
  libp2p:conn:out:QmTQqEj3 selecting crypto /secio/1.0.0 to 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +8ms
  mss:dialer	 (f7cgva) dialer select /secio/1.0.0 +0ms
  mss:dialer	 (f7cgva) writing multicodec: /secio/1.0.0 +0ms
  mss:dialer	 (f7cgva) received ack: /secio/1.0.0 +1ms
  libp2p:secio 1. propose - start +0ms
  libp2p:secio 1. propose - writing proposal +0ms
  libp2p:secio 1. propose - reading proposal <Buffer 0a 10 88 a5 ce 72 fc a2 60 f1 8e 65 26 2e 0b 38 d4 00 12 24 08 01 12 20 1a 00 c9 45 ef ff 59 dd f9 1e 56 8c 9c fd 98 5d 2d 50 f1 86 0d fa 27 cc 2a 0a ... > +2ms
  libp2p:secio 1.1 identify +0ms
Remote pubkey: CAESIBoAyUXv/1nd+R5WjJz9mF0tUPGGDfonzCoKdYNB726V
  libp2p:secio 1.1 identify - QmTQqEj3VFUqyxkQCcRYLixH3Y1qhYKNCrgUBW7HCsVoMp - identified remote peer as 12D3KooWBZsTt8cuGMRpEB4biGNRxG6GytBYY3tuGn29JF6RHmwA +0ms
  libp2p:secio 1.2 selection +0ms
  libp2p:secio 1. propose - finish +3ms
  libp2p:secio 2. exchange - start +0ms
  libp2p:secio 2. exchange - writing exchange +0ms
  libp2p:connection-manager checking limit of maxEventLoopDelay. current value: 3.2522586666666484 of Infinity +2s
  libp2p:connection-manager checking limit of maxEventLoopDelay. current value: 3.2107912499999856 of Infinity +2s
    ```
    