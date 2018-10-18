package protocol

type VMState struct {
  Chunks []Chunk     // virtual machine memory chunks
}

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState *VMState, txs []Transaction) VMState { panic("") }

func TendermintBlockProcessingExample() {
  // data
  var blocks   []Block    // Tendermint blockchain
  var vmStates []VMState  // virtual machine states

  // rules
  var k int               // some block number

  // ∀ k:
    assertEq(vmStates[k + 1], NextVMState(&vmStates[k], blocks[k].Txs))
}
