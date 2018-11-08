package protocol

type VM struct {
  Memory             []byte   // flat memory
  Stack              [][]byte // list of stack frames
  InstructionPointer int64    // pointer to the current instruction (WebAssembly code relative)
  ExecutedCounter    uint64   // total number of instructions executed by the VM since initialization
}

type WasmCode = []Chunk

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

func WasmEntryPoint(txs Transactions) { panic("") }

type Evidence = interface {}

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
) ComputationDispute { panic("") }

// presents a proof of the trace length
// [called by each party independently]
func (dispute ComputationDispute) PresentTraceLength(
  prevExecutedCounter uint64, prevProof MerkleProof,
  executedCounter uint64, proof MerkleProof,
  signature Seal,
) { panic("") }

// presents a hash of the VM state after executing the max common prefix of instructions
// [called by each party independently]
func (dispute ComputationDispute) PresentPrefixState(prefixVMHash Digest, signature Seal) { panic("") }

// presents an instruction pointer corresponding to the VM state after executing the common prefix
// [called by each party independently]
func (dispute ComputationDispute) PresentInstructionPointer(pointer int64, proof MerkleProof) { panic("") }