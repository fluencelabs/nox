#### IMPORTANT: State machine and README are under heavy development and can be outdated. Ask a question in gitter or file an issue.

# State machine

Server **State machine** for decentralized computation service.
State machine gets incoming transactions and queries from Tendermint running locally via **Tendermint ABCI** RPC calls.
So this State machine is **ABCI application** from Tendermint's point of view.

## Links
[Running local clusters in docker](/statemachine/docker/README.md)

[Launching Prometheus/Grafana monitoring](/tools/monitoring/README.md)

## Implementation details
State machine is a Scala application. It starts with a single optional parameter that describes a local TCP port
which the State machine should listen for ABCI RPC calls. [jTendermint](https://github.com/jTendermint/jabci) library
is used to as Java implementation of ABCI requests/responses.

[`ServerRunner`](/statemachine/src/main/scala/fluence/statemachine/ServerRunner.scala) is main class which opens
socket and registers `ABCIHandler` for processing RPC calls.

[`AbciHandler`](/statemachine/src/main/scala/fluence/statemachine/AbciHandler.scala) initializes different components
of the State machine and routes incoming ABCI requests to them.

Tendermint performs block formation and processes client requests in several threads. It maintains 3 of them to
establish 3 connections with ABCI application. The ABCI application is also expecting to process these connections in
dedicated threads which provided by `jTendermint` in case of the State machine. These connections are
* Mempool (for `CheckTx` request)
* Consensus (for `DeliverTx`, `Commit`, and other, yet unused requests)
* Query (for `Query` and other, yet unused requests)

The State machine follows Tendermint decomposition of client interactions to mutable **transactions** and read-only
**queries**. See Tendermint documentation for detailed understanding.

Following Tendermint requirements, the State machine maintains 3 dedicated **states** corresponding to available
RPC connections:
* Consensus state is 'real-time' state which updated each time when some transaction **applied**.
* Mempool state is the latest committed state. It is copied from current Consensus state during `Commit` processing.
Therefore any `CheckTx` of the *same transaction between 2 adjacent commits* processed the same way regardless any
`DeliverTx` processing between those commits. This allows to reach some degree of determinism regarding approving or
rejecting transactions by `CheckTx`.
* Query state is the committed state preceding the latest one. It is copied from previous Mempool state during `Commit`.
This allows to client to perform verification of the returned data if it is combined with merkle proof (because to
verify some block we need to wait for the next one).

`AbciHandler` routes incoming ABCI requests to their corresponding entry points:
* [`TxProcessor`](/statemachine/src/main/scala/fluence/statemachine/tx/TxProcessor.scala) for `DeliverTx` requests.
* [`QueryProcessor`](/statemachine/src/main/scala/fluence/statemachine/state/QueryProcessor.scala) for `Query` requests.
* [`Committer`](/statemachine/src/main/scala/fluence/statemachine/state/Committer.scala) for `Commit` requests.

A specific *state* is an immutable merkelized tree.
[`TreeNode`](/statemachine/src/main/scala/fluence/statemachine/tree/TreeNode.scala) implements this tree, whereas
[`MutableStateTree`](/statemachine/src/main/scala/fluence/statemachine/state/MutableStateTree.scala) implements mutable
wrapper for the root of Consensus state.

The State machine expects that any
[transaction](/statemachine/src/main/scala/fluence/statemachine/tx/Transaction.scala) consists of the following
components:
* Client ID
* Session ID
* Session counter
* Payload
* Signature (concatenation of other components signed by client's private key)

The possible immediate outcomes of processing a transaction are:
* Transaction rejected by `CheckTx` during
[parsing](/statemachine/src/main/scala/fluence/statemachine/tx/TxParser.scala) – because its format is wrong,
it duplicates some previous transaction or `client` and `signature` are inconsistent with each other.
* Transaction rejected by `DeliverTx` parser by the same reason – note that `CheckTx` would 'pass over' some transaction
that duplication another transaction from the same block because `CheckTx` uses 'lagged' Mempool state. Also note that
despite rejecting it is already including in the current block by Tendermint and will remain stored there in future.
* Transaction passed parsing and *queued* because it cannot be applied until the *previous* transaction (transaction
with the preceding counter value) applied. A transaction might be in a queued state arbitrarily long time – for example.
* Transaction passed parsing and applied at the same time. A successful transaction applying might cause to apply its
dependent transactions subsequently. See
[`TxProcessor`](/statemachine/src/main/scala/fluence/statemachine/tx/TxProcessor.scala) implementation for details.
