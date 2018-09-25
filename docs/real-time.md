## Real-time computations

### Overview

#### Real-time clusters

The _real-time component_ is the first out of two major data processing components in the network, and is also the only component a client directly interacts with. It consists of a bunch of modestly sized real-time clusters (expected typical size `4-16` nodes). Every node in the cluster runs the same WebAssembly code and stores the same data, while across different clusters the code and stored data generally differ.

A WebAssembly backend developer can designate certain functions as _entry points_, which means they are exposed through an API and can be invoked by a client. The real-time cluster takes care of marshaling and validating requests, leaving the developer only the task to properly implement the backend domain logic.

Every request that performs an entry point invocation is replicated across the nodes in the cluster. Each node applies it to the internal state and tries to reach (_BFT_) consensus with other nodes how the state should advance. This way the state is kept in sync across the nodes in the cluster. Every node that accepted the state change bears the full responsibility if it turns out the state change was incorrect. If the node doesn't agree with the state change, it's free to raise a _dispute_ right away.

Real-time clusters have a _data locality_ property: all the data required to perform the computation is stored locally on each node. This means real-time processing avoids network delays associated with identifying which chunk of data to fetch and then transferring it over the network. It's worth mentioning that replicating requests across the cluster or achieving consensus on the state change is also affected by network delays. However, this can be done in batch mode and doesn't affect latencies that much compared to non-trivial data processing.

#### Surrounding ecosystem

Real-time clusters composition is supposed to be stable and not change much over time. Before joining the cluster, each node places a security deposit with the corresponding Ethereum smart contract. The deposit is released to a node only after passing of a cooling-off period during which the correctness of node's operations may be disputed. Relative stability of real-time clusters means rare costly state resynchronizations which happen when nodes join or leave. 

However, this also means nodes in the cluster might eventually form a cartel producing incorrect computational results. Batch validation by independent randomly selected nodes is designated to prevent this. To make it possible, real-time nodes store the history of incoming requests and state changes in a _decentralized deep storage_ – [Swarm](https://swarm-guide.readthedocs.io). This history might be used by batch validators to inspect state transitions, but also – to restore the real-time cluster shall any of the nodes go down.

#### Client interactions

A client might interact with a real-time cluster through a predefined protocol. In general, the client is expected to be as light as possible and not store any data or domain logic code. To reason about security guarantees, the client normally performs the following (incomplete) list of checks:
- nodes participating in consensus and their security deposits in Ethereum smart contract
- the status of consensus over the state change and disputes presence
- whether the history of requests was properly updated in Swarm
- batch validation lag: how many requests the batch validation is behind the real-time


### Consensus engine

#### Tendermint

Internally, real-time clusters use [Tendermint](https://tendermint.com/docs/) as the BFT consensus framework, which is able to tolerate of up to `1/3` failed or Byzantine nodes. Every real-time node runs the Tendermint Core consensus engine and the Fluence state machine which connects the consensus engine with the WebAssembly VM.

Every request made by the client is turned into a transaction which is then sent to the `/broadcast_tx` endpoint in Tendermint. For example, if the deployed WebAssembly code was produced from this Rust snippet:
```rust
fn sum(x: i32, y: i32) -> i32 {
  x + y
}
```
then the client might send a transaction looking like `{'function': 'sum', 'x': 5, 'y': 3}`.

Before we dive deeper into the state machine, let's consider what Tendermint is responsible for in our case. Tendermint takes care of:
- replicating transactions across the cluster
- establishing a canonical order of transactions
- passing transactions to the state machine
- facilitating consensus on the state changes

However, there happen to be other requirements the state machine has to handle on it's own. We'll examine them below.

#### Happens-before relationship between transactions

First, we would like the client to be able to send a transaction that should be executed only after another transaction. In other words, there should be a support for the [_happens-before_](https://en.wikipedia.org/wiki/Happened-before) relationship between transactions. For example, let's imagine a client that checks stock quotes in a tight loop and based on this makes a decision whether to send a transaction into the Fluence network:

```
var max = ...
while(true) {
  sleep(1)
  
  val curr = nasdaq.ask("AAPL")
  if (curr > max) {
    max = curr
    fluence.send(tx = "{'function': 'update_high', 'symbol': 'AAPL', 'value': max}")
  }
}
```

In this example we assume a function `fn update_high(symbol: string, value: f64)` which updates a global maximum of the stock value is deployed as an entry point in the Fluence network.

Without transactions ordering, the global maximum might get updated incorrectly. One of the solutions would be to wait for a transaction to propagate into a block before sending another one, but this would limit available performance. Fluence state machine uses a session-based transactions ordering which is described in the corresponding section.
