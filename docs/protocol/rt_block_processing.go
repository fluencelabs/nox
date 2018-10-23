package protocol

type VMState struct {
  Chunks []Chunk     // virtual machine memory chunks
}

// applies block transactions to the virtual machine state to produce the new state
func NextVMState(vmState VMState, txs []Transaction) VMState { panic("") }

type Manifest struct {
  Header              Header        // block header
  VMStateHash         Digest        // virtual machine state hash after the previous block
  LastCommit          []Seal        // Tendermint nodes signatures for the previous block
  TxsReceipt          SwarmReceipt  // Swarm hash of the block transactions
  LastManifestReceipt SwarmReceipt  // Swarm hash of the previous manifest
}

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
