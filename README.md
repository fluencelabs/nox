# arqada
Pluggable relay infrastructure for p2p-enabled applications. Compatible with YJS, automerge, sync9, etc.

## Rust
```bash
make rust
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
make js
```

## Problems
1. JS disconnects from Rust after a while (perhaps because JS doesn't send ping-pongs)

