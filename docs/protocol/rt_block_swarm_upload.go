package protocol

type Manifest struct {
  Header              Header        // block header
  VMStateHash         Digest        // virtual machine state hash after the previous block
  LastCommit          []Seal        // Tendermint nodes signatures for the previous block
  TxsReceipt          SwarmReceipt  // Swarm hash of the block transactions
  LastManifestReceipt SwarmReceipt  // Swarm hash of the previous manifest
}

// creates a new manifest from the block and the previous block
func CreateManifest(block *Block, prevBlock *Block) Manifest { panic("") }

func TendermintBlockSwarmUploadExample() {
  // data
  var blocks    []Block            // Tendermint blockchain
  var vmStates  []VMState          // virtual machine states
  var manifests []Manifest         // manifests
  var swarm     map[Digest][]byte  // Swarm storage: hash(x) –> x

  // rules
  var k int                        // some block number

  // ∀ k:
    assertEq(manifests[k].Header, blocks[k].Header)
    assertEq(manifests[k].VMStateHash, MerkleRoot(vmStates[k].Chunks))
    assertEq(manifests[k].LastCommit, blocks[k].LastCommit)

    assertEq(manifests[k].TxsReceipt.ContentHash, SwarmHash(pack(blocks[k].Txs)))
    assertEq(manifests[k].LastManifestReceipt.ContentHash, SwarmHash(pack(manifests[k - 1])))

    assertEq(swarm[SwarmHash(pack(manifests[k]))], pack(manifests[k]))
    assertEq(swarm[SwarmHash(pack(blocks[k].Txs))], pack(blocks[k].Txs))
}
