## Computation machine

### Client interactions

As a reminder, for now we have been treating the virtual machine as a black box: transactions are being sent to this box input which changes state as a response:

```go
// produces the new state by applying block transactions to the old VM state
func NextVMState(code WasmCode, vmState VMState, txs Transactions) VMState {}
```

The client receives a segment of the virtual machine state along with the proof that the virtual machine state hash was stored in Swarm for future verification. The client also receives a proof that returned segment indeed corresponds to the state hash.

```go
type QueryResponse struct {
  Chunks    map[int]Chunk       // selected virtual machine state chunks
  Proofs    map[int]MerkleProof // Merkle proofs: chunks belong to the virtual machine state
  Manifests [3]Manifest         // block manifests
}
```

There are few potential vectors of attack on this design by malicious real-time cluster nodes. 

First, the cluster might silently drop a transaction sent by the client. This is more of a liveness issue (the cluster might keep dropping transactions forever) and in this protocol we focus more on safety. However, the client should be able to tell whether the transaction was executed or not.

Malicious cluster might also attempt to execute transactions sent by the client in the wrong order which might have critical consequences. For example, if some account balance is `0`, executing transactions `deposit(100), withdraw(50)` will have quite different results compared to `withdraw(50), deposit(100)` assuming we don't allow negative balances and the withdrawal operation will fail in the latter case.

Another potential case is malicious cluster adding a counterfeit transaction to the block – a transaction which wasn't sent by the valid client. This transaction might modify the virtual machine state in the way affecting results returned to honest clients.

Finally, a cluster might execute a transaction incorrectly: for a transaction `sum(5, 3)` it might return `4`. There is no way for the client to learn this in the real-time, but batch validation should recognize incorrect computations and discipline a misbehaving cluster.

To deal with the last attack, a verification game concept is used, which we will consider it in the next section.

### Verification game

Batch validators, as well as real-time nodes advance the virtual machine state by applying a block of transactions to it. As we remember from [§ Block processing](#block-processing), every manifest saved by the real-time cluster to Swarm carries the hash of the virtual machine state as well as the hash of the transactions block also uploaded to Swarm.
  
```go
func ProcessBlock(code WasmCode, block Block, prevVMState VMState, ...) {
  var vmState = NextVMState(code, prevVMState, block.Txs)
  var txsReceipt = SwarmUpload(pack(block.Txs))
   
  var manifest = Manifest{
    VMStateHash: MerkleRoot(vmState.Chunks),
    TxsReceipt:  txsReceipt,
    ...
  }  
  SwarmUpload(pack(manifest))
}
```

The WebAssembly code actually performing the state advance operation is uploaded to Swarm and registered in the Ethereum smart contract.

```go
type WasmFluenceContract struct {
  CodeReceipt SwarmReceipt // Swarm receipt for the stored WebAssembly code
  Initialized bool         // flag indicating whether the code is initialized
}

func (contract WasmFluenceContract) Init(receipt SwarmReceipt) {
  if !contract.Initialized {
    contract.CodeReceipt = receipt
  }
}

func DeployCode(code WasmCode, contract WasmFluenceContract) {
  var receipt = SwarmUpload(pack(code))
  contract.Init(receipt)
}
```

Internally, `WasmCode` might consist of multiple different WebAssembly modules, but it exposes a single function to the outside – a function accepting a block of transactions as an argument. This function doesn't return any results: it merely modifies the virtual machine state which can be queried later on.

```go
func WasmEntryPoint(txs Transactions) {}
```

Let's assume we've got two manifests: one carrying the previous virtual machine state hash, another – referencing the transactions block and having the new virtual machine state hash. Each manifest, as we have described [earlier](#query-response-verification), is certified by the signatures of the real-time cluster nodes.

<p align="center">
  <img src="images/state_advance.png" alt="State Advance" width="606px"/>
</p>

Real-time nodes that have properly signed the chain of manifests assert it's expected that passing _Txs<sub>k+1</sub>_ to the entry point method of the virtual machine having the state _VMState<sub>k</sub>_ will change the state to _VMState<sub>k+1</sub>_. If a batch validator or another real-time node doesn't agree with the state advance, a verification game is initiated.

Let's assume that node **A** performed _WasmCode_, _VMState<sub>k</sub>_, _Txs<sub>k+1</sub>_  —> _VMState<sub>k</sub><sup>A</sup>_ state change, and node **B** – _WasmCode_, _VMState<sub>k</sub>_, _Txs<sub>k+1</sub>_  —> _VMState<sub>k</sub><sup>B</sup>_. Any node can initiate a dispute by calling an Ethereum smart contract and passing to it hashes of the virtual machine states and transactions block.

```go
type ComputationDispute struct {
  wasmCodeHash Digest // Swarm hash of the WebAssembly code
  prevVMHash   Digest // hash of the previous virtual machine state
  txsHash      Digest // Swarm hash of the transactions block
  vmHashA      Digest // hash of the next virtual machine state as computed by the node `A`
  vmHashB      Digest // hash of the next virtual machine state as computed by the node `B`
}

func (contract WasmFluenceContract) OpenDispute(
  prevVMHash Digest, txsHash Digest, vmHashA Digest, vmHashB Digest) ComputationDispute {}
```












 