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

Finally, a cluster might execute a transaction incorrectly: for a transaction `sum(5, 3)` it might return `4`. There is no way for the client to learn this in the real-time, but batch validation should recognize incorrect computations and discipline a misbehaving cluster. To deal with this attack, a verification game is used, which we will consider it in the next few sections.

### Virtual machine

Virtual machine state which we previously has been treating as a sequence of bytes internally consists of few distinct parts. For the purposes of this protocol we consider only memory, stack, current instruction pointer and executed instructions counter.

```go
type VM struct {
  Memory             []byte   // flat memory
  Stack              [][]byte // list of stack frames
  InstructionPointer int64    // pointer to the current instruction (WebAssembly code relative)
  ExecutedCounter    uint64   // total number of instructions executed by the VM since initialization
}
```

Every part of the virtual machine can be serialized into a byte array so the entire `VM` will get serialized into a sequence of bytes. Because we can build the Merkle tree from this sequence using the `MerkleRoot` function, we can have Merkle proofs generated for any part of the virtual machine – for example, for the executed instructions counter.

### Verification game

#### WebAssembly package

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

#### Dispute initiation

Let's assume we've got two manifests: one carrying the previous virtual machine state hash, another – referencing the transactions block and having the new virtual machine state hash. Each manifest, as we have described [earlier](#query-response-verification), is certified by the signatures of the real-time cluster nodes: for the <code>Manifest<sub>k</sub></code>, linked <code>Manifest<sub>k+1</sub></code> and <code>Manifest<sub>k+2</sub></code> are required to verify this certification.

<p align="center">
  <img src="images/state_advance.png" alt="State Advance" width="606px"/>
</p>

Real-time nodes that have properly signed the chain of manifests assert it's expected that passing <code>Txs<sub>k+1</sub></code> to the entry point method of the virtual machine having the state <code>VMState<sub>k</sub></code> will change the state to <code>VMState<sub>k+1</sub></code>. If a batch validator or another real-time node doesn't agree with the state advance, a verification game is initiated.

Let's assume that node **A** performed <code>WasmCode, VMState<sub>k</sub>, Txs<sub>k+1</sub> —> VMState<sub>k+1</sub><sup>A</sup></code> state change, and node **B** – <code>WasmCode, VMState<sub>k</sub>, Txs<sub>k+1</sub> —> VMState<sub>k+1</sub><sup>B</sup></code>. Any node can initiate a dispute by calling an Ethereum smart contract and passing to it hashes of the virtual machine states and transactions block.

```go
type ComputationDispute struct {
  wasmCodeHash Digest   // Swarm hash of the WebAssembly code
  prevVMHash   Digest   // hash of the previous virtual machine state
  txsHash      Digest   // Swarm hash of the transactions block
  vmHashA      Digest   // hash of the next virtual machine state as computed by the node `A`
  vmHashB      Digest   // hash of the next virtual machine state as computed by the node `B`
  evidenceA    Evidence // evidence that the node `A` has really performed declared transition
  evidenceB    Evidence // evidence that the node `B` has really performed declared transition
}

// opens a computation dispute
// [can be called by any party with enough evidence gathered]
func (contract WasmFluenceContract) OpenDispute(
  prevVMHash Digest,
  txsHash Digest,
  vmHashA Digest, evidenceA Evidence,
  vmHashB Digest, evidenceB Evidence,
) ComputationDispute {}
```

Note that the contract requires an evidence that the node has really performed the transition <code>WasmCode, VMState<sub>k</sub>, Txs<sub>k+1</sub> —> VMState<sub>k+1</sub><sup>X</sup></code> to be submitted for each node **X**. This evidence might be different for different cases – for batch validators it will merely be a digital signature of the performed transition: 

```go
// confirms that the transition from the previous virtual machine state to the next state is correct  
func (validator BatchValidator) ConfirmTransition(
  prevVMHash Digest, vmHash Digest, txsHash Digest) Seal {
    return Sign(
      validator.PublicKey, 
      validator.privateKey,
      Hash(pack(prevVMHash, vmHash, txsHash)),
    ) 
}
```

For real-time nodes gathering the evidence is a bit more involved. Remember that two additional manifests: <code>Manifest<sub>k+1</sub></code> and <code>Manifest<sub>k+2</sub></code> are required to prove that the node has signed the <code>Manifest<sub>k</sub></code> which references state <code>VMState<sub>k</sub></code> and transactions block <code>Txs<sub>k</sub></code>. This means 4 manifests with indices ranging from `k` to `k+3` are required for the evidence:

- <code>Manifest<sub>k</sub></code> references <code>VMState<sub>k</sub></code>
- <code>Manifest<sub>k+1</sub></code> references <code>VMState<sub>k+1</sub></code> and <code>Txs<sub>k+1</sub></code>
- <code>Manifest<sub>k+1</sub></code> and <code>Manifest<sub>k+2</sub></code> carry the real-time node **X** signature verifying <code>Manifest<sub>k</sub></code>
- <code>Manifest<sub>k+2</sub></code> and <code>Manifest<sub>k+3</sub></code> carry the real-time node **X** signature verifying <code>Manifest<sub>k+1</sub></code>

**FIXME:** an evidence should also confirm the version of WebAssembly code used to perform the state transition.

#### Trace length mismatch resolution

To begin the dispute resolution, each side has to first present the length of an execution trace to the smart contract. This can be done by submitting <code>(VM<sub>k</sub>.ExecutedCounter,  VM<sub>k+1</sub>.ExecutedCounter)</code> with required Merkle proofs so the smart contract could compute the difference.

```go
// presents a proof of the trace length
// [called by each party independently]
func (dispute ComputationDispute) PresentTraceLength(
  prevExecutedCounter uint64, prevProof MerkleProof,
  executedCounter uint64, proof MerkleProof,
  signature Seal,
) {}
```
  
It might happen that <code>VM<sub>k+1</sub><sup>A</sup>.ExecutedCounter</code> is not equal to <code>VM<sub>k+1</sub><sup>B</sup>.ExecutedCounter</code>, which means one of the nodes had the longer execution trace than another. This situation requires additional processing, which we consider below.

Let's denote the state of the virtual machine after processing `p` WebAssembly instructions while executing the transactions block <code>Txs<sub>k</sub></code> as <code>VM[p]<sub>k</sub></code>. Let's also denote the total number of instructions executed by **A** while processing <code>Txs<sub>k</sub></code> as `n`, **B** – as `m`, where `n < m`.

<p align="center">
  <img src="images/trace_mismatch.png" alt="Trace Mismatch" width="745px"/>
</p>

Now we are interested whether the state <code>VM[n]<sub>k+1</sub><sup>A</sup></code> is equal to the state <code>VM[n]<sub>k+1</sub><sup>B</sup></code>. Smart contract requires both sides to present the hash of the state <code>VM[n]<sub>k+1</sub></code>:

```go
// presents a hash of the VM state after executing the max common prefix of instructions
// [called by each party independently]
func (dispute ComputationDispute) PresentPrefixState(prefixVMHash Digest, signature Seal) {}
```

If <code>VM[n]<sub>k+1</sub><sup>A</sup> != VM[n]<sub>k+1</sub><sup>B</sup></code>, we have a mismatch between the virtual machine states computed using the same number of instructions. This means we can initiate a prefix dispute between those states and use a standard verification game to resolve it. Note that instructions in the execution trace prefixes might be completely different between **A** and **B** – everything that matters is the same length of those prefixes.

If <code>VM[n]<sub>k+1</sub><sup>A</sup> == VM[n]<sub>k+1</sub><sup>B</sup></code>, then we need to figure out whether the decision of **A** to halt the program after `n` instructions, as well as the decision of **B** to run another `n+1` instruction were correct. To do so, we require both nodes to present the instruction pointer (along with the Merkle proof) corresponding to the state <code>VM[n]<sub>k+1</sub></code> to the smart contract. If it's negative, it means the execution should have stopped which means **B** misbehaved, otherwise the execution should have continued and **A** should be penalized.

```go
// presents an instruction pointer corresponding to the VM state after executing the common prefix
// [called by each party independently] 
func (dispute ComputationDispute) PresentInstructionPointer(pointer int64, proof MerkleProof) {}
```

#### Verification game












