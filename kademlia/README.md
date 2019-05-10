# Fluence Kademlia

Implementation of S/Kademlia, with a few additions.

## Kademlia Core

- Entry point: `Kademlia` trait
- Protocol: `Key` for keys, `KademliaRpc` to express RPC calls, `Node` for key+contact
- State: `Bucket`, `Siblings`, all wrapped with `RoutingState` -- use just it
- Routing: `LocalRouting` for local table lookups, `IterativeRouting` for network-wise algorithms; use just `RoutingTable`
- Extension: `StoredRoutingState` to persist and bootstrap the RoutingState
- Extension: `RefreshingIterativeRouting` to update untouched buckets once in a while

## Kademlia HTTP transport

TODO: 
- how to launch Kademlia with HTTP transport
- HTTP protocol for client implementations

## Simulations

TODO