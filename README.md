# arqada
Pluggable relay infrastructure for p2p-enabled applications. Compatible with YJS, automerge, sync9, etc.

## Rust
```bash
RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info" cargo run
```

It will print something like
```
peer id: QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
Public Key: /mIoVN5TXRpdorh9+5Mts7I3b/zSQ87NdQtVY9JApc0=
PrivateKey: /O5p1cDNIyEkG3VP+LqozM+gArhSXUdWkKz6O+C6Wtr+YihU3lNdGl2iuH37ky2zsjdv/NJDzs11C1Vj0kClzQ==
[2019-12-23T14:53:28Z TRACE libp2p_dns] Created a CpuPoolResolver
[2019-12-23T14:53:28Z DEBUG libp2p_tcp] Listening on "/ip4/127.0.0.1/tcp/30000"
Listening on /ip4/127.0.0.1/tcp/30000/p2p/QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
External address: None
Local peer id QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
```


## JS

- `QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw` from Rust becomes `12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G` in JS
- Now `12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G` is hardcoded in run.js

Run like this:
```bash
cd arqada/js
DEBUG="*,-latency-monitor:LatencyMonitor" node run.js
```

## Problems
### JS and Rust both hangs
#### Rust
```
arqada$ RUST_LOG="trace,tokio_threadpool=info,tokio_reactor=info,mio=info" cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.14s
     Running `target/debug/arqada`
peer id: QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
Public Key: /mIoVN5TXRpdorh9+5Mts7I3b/zSQ87NdQtVY9JApc0=
PrivateKey: /O5p1cDNIyEkG3VP+LqozM+gArhSXUdWkKz6O+C6Wtr+YihU3lNdGl2iuH37ky2zsjdv/NJDzs11C1Vj0kClzQ==
[2019-12-23T14:53:28Z TRACE libp2p_dns] Created a CpuPoolResolver
[2019-12-23T14:53:28Z DEBUG libp2p_tcp] Listening on "/ip4/127.0.0.1/tcp/30000"
Listening on /ip4/127.0.0.1/tcp/30000/p2p/QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
External address: None
Local peer id QmTESkr2vWDCKqiHVsyvf4iRQCBgvNDqBJ6P3yTTDb6haw
[2019-12-23T14:53:30Z TRACE libp2p_tcp] Incoming connection from /ip4/127.0.0.1/tcp/65410 at /ip4/127.0.0.1/tcp/30000
something happened in swarm runloop
[2019-12-23T14:53:30Z TRACE multistream_select::protocol] Received message: Header(V1)
[2019-12-23T14:53:30Z TRACE multistream_select::protocol] Received message: Protocol(Protocol(b"/secio/1.0.0"))
[2019-12-23T14:53:30Z DEBUG multistream_select::listener_select] Listener: confirming protocol: /secio/1.0.0
[2019-12-23T14:53:30Z DEBUG multistream_select::listener_select] Listener: sent confirmed protocol: /secio/1.0.0
[2019-12-23T14:53:30Z DEBUG libp2p_secio] Starting secio upgrade
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] agreements proposition: P-256,P-384
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] ciphers proposition: AES-128,AES-256,TwofishCTR
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] digests proposition: SHA256,SHA512
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] starting handshake; local nonce = [56, 238, 135, 180, 18, 56, 29, 36, 22, 106, 21, 184, 48, 166, 80, 73]
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] sending proposition to remote
[2019-12-23T14:53:30Z TRACE tokio_io::framed_read] attempting to decode a frame
[2019-12-23T14:53:30Z TRACE tokio_io::framed_read] frame decoded from buffer
[2019-12-23T14:53:30Z DEBUG libp2p_secio::handshake] selected cipher: Aes256
[2019-12-23T14:53:30Z DEBUG libp2p_secio::handshake] selected hash: Sha256
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] received proposition from remote; pubkey = Rsa(PublicKey([48, 130, 1, 10, 2, 130, 1, 1, 0, 164, 116, 52, 36, 116, 148, 184, 127, 252, 192, 4, 202, 80, 207, 146, 150, 39, 222, 183, 16, 49, 85, 174, 210, 233, 11, 238, 30, 224, 0, 82, 204, 249, 125, 67, 14, 181, 54, 101, 139, 216, 106, 236, 84, 120, 108, 15, 62, 115, 201, 231, 154, 76, 162, 134, 3, 160, 240, 221, 151, 108, 243, 189, 183, 218, 72, 233, 90, 61, 155, 119, 158, 117, 244, 204, 64, 145, 175, 156, 38, 35, 174, 40, 198, 115, 184, 162, 243, 38, 152, 146, 107, 84, 26, 124, 215, 70, 193, 205, 200, 33, 18, 212, 22, 69, 37, 4, 200, 13, 80, 174, 202, 108, 247, 180, 48, 39, 27, 73, 243, 248, 27, 13, 175, 2, 174, 168, 124, 160, 34, 146, 126, 211, 28, 163, 21, 229, 17, 239, 78, 178, 10, 156, 1, 113, 91, 201, 129, 131, 184, 227, 81, 99, 15, 209, 231, 179, 250, 6, 213, 109, 229, 213, 87, 12, 214, 216, 29, 189, 4, 57, 154, 100, 44, 227, 227, 153, 250, 142, 31, 29, 71, 237, 231, 173, 193, 18, 34, 168, 114, 86, 28, 87, 212, 129, 101, 205, 1, 197, 166, 58, 142, 227, 252, 18, 50, 77, 193, 57, 55, 235, 221, 81, 52, 83, 63, 81, 162, 103, 80, 134, 201, 63, 241, 19, 99, 6, 90, 195, 49, 186, 216, 220, 131, 18, 49, 9, 237, 98, 6, 62, 152, 8, 45, 144, 138, 190, 55, 250, 171, 68, 130, 36, 201, 85, 143, 2, 3, 1, 0, 1])); nonce = [114, 103, 145, 245, 158, 7, 59, 8, 165, 153, 150, 164, 87, 109, 240, 214]
[2019-12-23T14:53:30Z TRACE libp2p_secio::handshake] sending exchange to remote
[2019-12-23T14:53:30Z TRACE tokio_io::framed_read] attempting to decode a frame
[2019-12-23T14:53:50Z DEBUG libp2p_core::transport::timeout] timeout elapsed for connection
[2019-12-23T14:53:50Z DEBUG libp2p_tcp] Dropped TCP connection to V4(127.0.0.1:65410)
something happened in swarm runloop
```

#### JS
```
js$ DEBUG="*,-latency-monitor:LatencyMonitor,-libp2p:connection-manager" node run.js
Peer created QmUymnGmx7xEH7cfFSfoAXxztb6M6JbZmLuPmhztuH43Ee
  libp2p:switch:dialer create: 8 peer limit, 30000 dial timeout +0ms
  libp2p:switch:transport adding Circuit +0ms
  libp2p libp2p is starting +0ms
  libp2p:switch:transport adding TCP +4ms
  libp2p:switch The switch is starting +0ms
  libp2p:tcp:listen Listening on 0 127.0.0.1 +0ms
  libp2p:switch The switch has started +18ms
  libp2p libp2p has started +24ms
node started
will dial /ip4/127.0.0.1/tcp/30000/p2p/12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G
Discovered: 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G
  libp2p connecting to discovered peer +2ms
  libp2p:switch:dial starting dial queue to 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +0ms
  libp2p:conn:out:QmUymnGm dialing 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +0ms
  libp2p:conn:out:QmUymnGm dialing transport TCP +1ms
  libp2p:switch:transport dialing TCP [ '/ip4/127.0.0.1/tcp/30000/ipfs/12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G' ] +30ms
  libp2p:switch:dialer dialMany:start +41ms
  libp2p:switch:dialer dialSingle: 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G:/ip4/127.0.0.1/tcp/30000/ipfs/12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +1ms
  libp2p:switch:dialer:queue work:start +0ms
  libp2p:tcp:dial Connecting to 30000 127.0.0.1 +0ms
  libp2p:switch:dialer:queue work:success +2ms
  libp2p:switch:dialer dialMany:success +4ms
  libp2p:conn:out:QmUymnGm successfully dialed 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +10ms
  mss:dialer	 (bkumef) dialer handle conn +0ms
  mss:dialer	 (bkumef) writing multicodec: /multistream/1.0.0 +1ms
  mss:dialer	 (bkumef) received ack: /multistream/1.0.0 +4ms
  mss:dialer	 (bkumef) handshake success +1ms
  libp2p:conn:out:QmUymnGm selecting crypto /secio/1.0.0 to 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +8ms
  mss:dialer	 (bkumef) dialer select /secio/1.0.0 +0ms
  mss:dialer	 (bkumef) writing multicodec: /secio/1.0.0 +0ms
  mss:dialer	 (bkumef) received ack: /secio/1.0.0 +2ms
  libp2p:secio 1. propose - start +0ms
  libp2p:secio 1. propose - writing proposal +0ms
  libp2p:secio 1. propose - reading proposal <Buffer 0a 10 38 ee 87 b4 12 38 1d 24 16 6a 15 b8 30 a6 50 49 12 24 08 01 12 20 fe 62 28 54 de 53 5d 1a 5d a2 b8 7d fb 93 2d b3 b2 37 6f fc d2 43 ce cd 75 0b ... > +2ms
  libp2p:secio 1.1 identify +0ms
Remote pubkey: CAESIP5iKFTeU10aXaK4ffuTLbOyN2/80kPOzXULVWPSQKXN
  libp2p:secio 1.1 identify - QmUymnGmx7xEH7cfFSfoAXxztb6M6JbZmLuPmhztuH43Ee - identified remote peer as 12D3KooWSwNXzEeGjgwEocRJBzbdoDqxbz3LdrwgSuKmKeGvbM4G +1ms
  libp2p:secio 1.2 selection +0ms
  libp2p:secio 1. propose - finish +3ms
  libp2p:secio 2. exchange - start +0ms
  libp2p:secio 2. exchange - writing exchange +0ms
```