# Real-time computations

## Overview

### Real-time clusters

The _real-time component_ is the first out of two major data processing components in the network, and is also the only component a client directly interacts with. It consists of a bunch of modestly sized real-time clusters (expected typical size `4-16` nodes). Every node in the cluster runs the same WebAssembly code and stores the same data, while across different clusters the code and stored data generally differ.

A WebAssembly backend developer can designate certain functions as _entry points_, which means they are exposed through an API and can be invoked by a client. The real-time cluster takes care of marshaling and validating requests, leaving the developer only the task to properly implement the backend domain logic.

Every request that performs an entry point invocation is replicated across the nodes in the cluster. Each node applies it to the internal state and tries to reach (_BFT_) consensus with other nodes how the state should advance. This way the state is kept in sync across the nodes in the cluster. Every node that accepted the state change bears the full responsibility if it turns out the state change was incorrect. If the node doesn't agree with the state change, it's free to raise a _dispute_ right away.

Real-time clusters have a _data locality_ property: all the data required to perform the computation is stored locally on each node. This means real-time processing avoids network delays associated with identifying which chunk of data to fetch and then transferring it over the network. It's worth mentioning that replicating requests across the cluster or achieving consensus on the state change is also affected by network delays. However, this can be done in batch mode and doesn't affect latencies that much compared to non-trivial data processing.

### Surrounding ecosystem

Real-time clusters composition is supposed to be stable and not change much over time. Before joining the cluster, each node places a security deposit with the corresponding Ethereum smart contract. The deposit is released to a node only after passing of a cooling-off period during which the correctness of node's operations may be disputed. Relative stability of real-time clusters means there will be less expensive state resynchronizations which happen when nodes join or leave. 

However, this also means nodes in the cluster might eventually form a cartel producing incorrect computational results. Batch validation by independent randomly selected nodes outside of the cluster is designated to prevent this. To make it possible, real-time nodes store the history of incoming requests and state changes in a _decentralized deep storage_ – [Swarm](https://swarm-guide.readthedocs.io). This history might be used by batch validators to inspect state transitions, but also – to restore the real-time cluster shall any of the nodes go down.

### Client interactions

A client might interact with a real-time cluster through a predefined protocol. In general, the client is expected to be as light as possible and not store any data or domain logic code. To reason about security guarantees, the client normally performs the following (incomplete) list of checks:
- nodes participating in consensus and their security deposits in Ethereum smart contract
- the status of consensus over the state change and disputes presence
- whether the history of requests was properly updated in Swarm
- batch validation lag: how many requests the batch validation is behind the real-time

## API

From the client point of view the real-time cluster API is fairly simple. Let's imagine that a WebAssembly backend developer has deployed the WASM code, for example the one compiled from this Rust snippet:

```rust
static mut AMOUNT: u32 = 0;
static mut MAXIMUM: f64 = 0.0;

// #[no_mangle] directive tells the compiler to avoid changing the method name

#[no_mangle]
pub unsafe fn set_amount(amount: u32) {
  AMOUNT = amount;
}

#[no_mangle]
pub unsafe fn get_amount() -> u32 {
  AMOUNT
}

#[no_mangle]
pub unsafe fn update_maximum(value: f64) -> f64 {
  if value > MAXIMUM {
    MAXIMUM = value;
  }
  MAXIMUM  
}

#[no_mangle]
pub fn sum(x: u32, y: u32) -> u32 {
  x + y
}
```

Let's consider how the client could call those functions and receive the results. Assuming the client already has a connection to the real-time cluster, the first thing the client has to do is to create a new session:

```javascript
var fluence = ...
var session = fluence.newSession();
```

Once the session is initialized, it can be used to invoke various functions:

```javascript

session.invoke("set_amount", [50]);
session.invoke("set_amount", [10]);
session.invoke("set_amount", [35]);
var c = await session.invoke("get_amount");              // c == 35

session.invoke("update_maximum", [10.2]);
session.invoke("update_maximum", [75.0]);
var m = await session.invoke("update_maximum", [50.5]);  // m == 75.0

var s = await session.invoke("sum", [5, 3]);             // s == 8
```

In this example we can observe _effectful_ functions modifying the global state, functions returning some value which is a part of the global state, functions modifying the global state and returning some value and finally, _pure_ functions like `sum` that are deterministic and not accessing the global state at all.

Sometimes the client might want to invoke the function only for its _side effects_ – that's what we see when the client invokes the `set_amount` function. In this case, there is no need to use `await` to retrieve results. Moreover, the absence of awaiting for results allows the session to efficiently batch/parallelize sent requests and should be used whenever possible.

It's also possible to await for the entire session itself using `await session.sync()`. This will await for all invocations previously made in the session to complete:

```javascript
session.invoke("set_amount", [50]);
session.invoke("set_amount", [10]);
session.invoke("set_amount", [35]);

await session.sync();
```

### Sessions cleaning

Once the client is done with the session, it should be closed to release the connection – normally by using a `try/finally` block:

```javascript
var session = fluence.newSession();
try {
  session.invoke("set_amount", [50]);
  ...
  var s = await session.invoke("sum", [5, 3]);
} finally {
  session.close();
}
```

### Function invocations ordering

It's important to mention that while function calls might get parallelized, there is a total order between invocations made within a single session. For example, there is a guarantee that calls `set_amount(50)`, `set_amount(10)` and `set_amount(35)` will be run exactly in this order, and `get_amount` will be executed the last and return `35`. 

However, there is no order between function calls made from different sessions. In general, ordering behavior is similar to the [happens-before semantic](https://docs.oracle.com/javase/specs/jls/se7/html/jls-17.html#jls-17.4.5) in Java Memory Model if we replace a notion of session with a JVM thread. In future some synchronization primitives might be provided to allow better concurrent programming with different sessions.

### Error handling

If any function execution has failed, then the rest of the functions invoked after in the session will not be executed at all. The failed function execution will not be rolled back which means the virtual machine might be left in the inconsistent state. The client will receive an exception on the first `await` after the failed function. After that, the session will not be usable anymore and should be closed.

<img src="images/symbols/twemoji-exclamation.png" width="24px"/> **TODO:** _Potentially, we should roll back the failed function and restore the virtual machine state to the point where it was before beginning the execution of such function._

For example, consider this backend:

```rust
static mut COUNTER: u32 = 0;

#[no_mangle]
pub unsafe fn inc() {
  COUNTER += 1;
}

#[no_mangle]
pub unsafe fn fail() {
  COUNTER -= 1;
  panic!("Let's fail!");
  COUNTER += 1;           // unreachable
}
```

and this client code:

```javascript

session.invoke("inc", []);    // COUNTER == 1
session.invoke("inc", []);    // COUNTER == 2

session.invoke("fail", []);   // this call will fail and leave COUNTER == 1

session.invoke("inc", []);    // this call won't execute
session.invoke("inc", []);    // this call won't execute

try {
  await session.sync();       // this will throw the "Let's fail!" exception
} catch(err) {
  // ...
}

try {
  session.invoke("inc", []);  // this will throw a failed session state exception
} catch(err) {
  // ...
}
```

## Consensus engine

### Tendermint

Internally, real-time clusters use [Tendermint](https://tendermint.com/docs/) as the BFT consensus framework, which is able to tolerate of up to `1/3` failed or Byzantine nodes.

Every request made by the client is turned into a transaction which is then sent to one of Tendermint endpoints. For example, if the deployed WebAssembly code was produced from this Rust snippet: `fn sum(x: i32, y: i32) -> i32 { x + y }`, then the client might send a transaction looking like `{"fn": "sum", "args": [5, 3]}` and await until `8` will be retrieved as a result.

Tendermint takes care of:
- replicating transactions across the cluster
- establishing a canonical order of transactions
- passing transactions to the state machine
- facilitating consensus on the state changes

However, it doesn't invoke the WebAssembly code or verify clients signatures, which is done by the Fluence-specific state machine. 

### State machine

Every real-time node carries a state which is updated using transactions furnished through the consensus engine. If every transition made since the genesis was correct, we can expect that the state itself is correct too. Results obtained by querying such a state should be correct as well. However, if at any moment in time there was an incorrect transition, all subsequent states can potentially be incorrect even if all later transitions were correct.

Using Tendermint, cluster nodes reach consensus not only over the canonical order of transactions, but also over the Merkle root of the state – `app_hash` in Tendermint terminology. The client can obtain such Merkle root from any node in the cluster, verify cluster nodes signatures and check that more than `2/3` of the nodes have accepted the Merkle root change – i.e. that consensus was reached.

The state machine is not a part of Tendermint and normally has an application-specific logic. In Fluence network it's responsible for requests authentication, establishing the happens-before semantic between the client transactions and invoking deployed WebAssembly functions.

Each node in the cluster runs a Tendermint instance and a Fluence state machine instance with Tendermint connecting the nodes together.

### Tendermint reference

#### Blockchain

Tendermint combines transactions into ordered lists – blocks. Besides the transaction list, a block also has some metadata that helps to provide integrity and verifiability guarantees. This metadata consists of two major parts:

- metadata related to the current block
  - `height` – an index of this block in the blockchain
  - hash of the transaction list in the block
  - hash of the previous block
- metadata related to the previous block
  - `app_hash` – hash of the state machine state that was achieved at the end of the previous block
  - previous block _voting process_ information

<p align="center">
  <img src="images/tendermint_blockchain.png" alt="Tendermint Blockchain" width="721px"/>
</p>

To create a new block a single node – the _block proposer_ is chosen. The proposer composes the transaction list, prepares the metadata and initiates the voting process. Then other nodes make votes accepting or declining the proposed block. If enough number of votes accepting the block exists (i.e. the _quorum_ was achieved – more than `2/3` of the nodes in the cluster voted positively), the block is considered committed. Otherwise, another round of block creation is initiated.

Once the block is committed, every node's state machine applies newly committed block transactions to the state in the same order those transactions are present in the block. Once all block transactions are applied, the new state machine `app_hash` is memorized by the node and will be used in the next block formation.

Note that the information about the voting process and the `app_hash` achieved during the block processing by the state machine are not stored in the current block. The reason is that this part of metadata is not known to the proposer at time of block creation and becomes available only after successful voting. That's why the `app_hash` and voting information for the current block are stored in the next block. That metadata can be received by external clients only upon the next block commit event.

Once the block `k` is committed, only the presence and the order of its transactions is verified, but not the state achieved by their execution. Because the `app_hash` resulted from the block `k` execution is only available when the block `k + 1` is committed, the client has to wait for the next block to trust a result computed by the transaction in the block `k`. To avoid making the client wait if there were no transactions for a while, Tendermint makes the next block (possibly empty) in a short time after the previous block was committed.

#### ABCI

Application BlockChain Interface (ABCI) allows Tendermint to interact with the underlying application. There are two major types of interactions: transactions and queries.

When the client sends a transaction to Tendermint, it does not immediately receive a result of it's execution. Instead, the transaction is placed into the Tendermint mempool first and then combined with other transactions into a block which is eventually delivered to the application state machine. Once the state machine applies the block and updates the state, it can be queried using the Tendermint query API.

Queries issued to Tendermint are processed outside of the normal consensus flow. This means a malicious node might return an incorrect result to the unsuspicious client. To avoid this, a client might query multiple nodes and compare results. The other option is to demand that nodes always furnish a Merkle proof along with the result and check this proof against the `app_hash` – the Merkle root of the state.

Below we will consider in few more details how the Fluence state machine interacts with the Tendermint consensus engine.

## Fluence state machine

### Function invocation

The client is able to invoke a WebAssembly function and receive the result using a simple `await session.invoke("sum", [5, 3])` statement. Internally, this expands to multiple steps:

First, the client sends a transaction to the cluster. In addition to the function name and arguments, the transaction also contains the identifier of the client, the client session identifier and the ordering number of this function call within the session.
```
  {
    "fn": "sum", 
    "args": [5, 3], 
    "meta": {
      "client": "0xA425FB82",      
      "session": 117,
      "counter": 23
    }
  }
```

<img src="images/symbols/twemoji-exclamation.png" width="24px"/> **TODO**  
_It's unclear how the session identifier will be generated and who is responsible for that: the client code or the real-time cluster. It's also unclear whether the real-time cluster should resist bogus session identifiers potentially crafted by a malicious client._

### Happens-before relationship between transactions

We need the client to be able to send a transaction that should be executed only after another transaction. In other words, there should be a support for the [_happens-before_](https://en.wikipedia.org/wiki/Happened-before) relationship between transactions. For example, let's imagine a client that checks stock quotes in a tight loop and based on this makes a decision whether to send a transaction into the Fluence network:

```
max = ...
while(true) {
  sleep(1)
  
  curr = nasdaq.ask("AAPL")
  if (curr > max) {
    max = curr
    fluence.request("{'function': 'update_high', 'symbol': 'AAPL', 'value': max}")
  }
}
```

In this example we assume a function `fn update_high(symbol: &str, value: f64)` which updates a global maximum of the stock value is deployed as an entry point in the Fluence network.

Without transactions ordering, the global maximum might get updated incorrectly. One of the solutions would be to wait for a transaction to propagate into the block before sending another one, but this would limit available performance. Fluence state machine uses a session-based transactions ordering which is described in the corresponding section.

### RPC support

We want the client to be able to interact with the real-time cluster using a conventional request-response API: 

```
response = fluence.request("{'function': 'sum', 'x': 3, 'y': 5}")
print(response)  # prints "8"
```

Request-response API is not really native to Tendermint, so Fluence state machine wraps Tendermint to provide it to the client.

----
\> [Twemoji](https://twemoji.twitter.com/) graphics is made by Twitter, Inc and other contributors and is licensed under [CC-BY 4.0]( https://creativecommons.org/licenses/by/4.0/)
