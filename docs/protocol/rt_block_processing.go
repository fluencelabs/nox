package protocol

type VMState struct {
  Chunks []Chunk     // virtual machine memory chunks
}

// deserializes a byte array into the virtual machine state
func VMStateUnpack([]byte) VMState { panic("") }

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState VMState, txs []Transaction) VMState { panic("") }

type Manifest struct {
  Header              Header        // block header
  VMStateHash         Digest        // hash of the VM state derived by applying the block
  LastCommit          []Seal        // Tendermint nodes signatures for the previous block header
  TxsReceipt          SwarmReceipt  // Swarm hash of the block transactions
  LastManifestReceipt SwarmReceipt  // Swarm hash of the previous manifest
}

// deserializes a byte array into the manifest
func ManifestUnpack([]byte) Manifest { panic("") }

// returns the new virtual machine state, the manifest for the stored block and the next app hash
func ProcessBlock(block Block, prevVMState VMState, prevManifestReceipt SwarmReceipt,
) (VMState, Manifest, SwarmReceipt, Digest) {
  var vmState = NextVMState(prevVMState, block.Txs)
  var txsReceipt = SwarmUpload(pack(block.Txs))

  var manifest = Manifest{
    Header:              block.Header,
    VMStateHash:         MerkleRoot(vmState.Chunks),
    LastCommit:          block.LastCommit,
    TxsReceipt:          txsReceipt,
    LastManifestReceipt: prevManifestReceipt,
  }
  var receipt = SwarmUpload(pack(manifest))
  var nextAppHash = Hash(pack(manifest))

  return vmState, manifest, receipt, nextAppHash
}
