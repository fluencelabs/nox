package protocol

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

type ComputationDispute struct {
  wasmCodeHash Digest // Swarm hash of the WebAssembly code
  prevVMHash   Digest // hash of the previous virtual machine state
  txsHash      Digest // Swarm hash of the transactions block
  vmHashA      Digest // hash of the next virtual machine state as computed by the node `A`
  vmHashB      Digest // hash of the next virtual machine state as computed by the node `B`
}

func (contract WasmFluenceContract) OpenDispute(
  prevVMHash Digest, txsHash Digest, vmHashA Digest, vmHashB Digest) ComputationDispute { panic("") }
