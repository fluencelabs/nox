# Janus
Pluggable relay infrastructure for p2p-enabled applications.

## How to build

```rust
make
```

## How to run

Run server:
```rust
make server
```

Run server with debug logs:
```rust
make server-debug
``` 

Run client:
```rust
make client
```

Run client with debug logs:
```rust
make client-debug args="multiaddr peer_id"
```

## Architecture
![*The architecture of Janus](https://raw.githubusercontent.com/fluencelabs/arqada/master/janus/img/janus_arch.png)
