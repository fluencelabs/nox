package protocol

import "reflect"
import "bytes"

type Manifest struct {
  Header                Header        // block header
  LastCommit            []Seal        // Tendermint nodes signatures for the previous block
  TxsSwarmHash          Digest        // Swarm hash of the block transactions
  VMStateHash           Digest        // virtual machine state hash after the previous block
  LastManifestSwarmHash Digest        // Swarm hash of the previous manifest
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
    assert(manifests[k].Header == blocks[k].Header)
    assert(reflect.DeepEqual(manifests[k].LastCommit, blocks[k].LastCommit))
    assert(manifests[k].TxsSwarmHash == SwarmHash(pack(blocks[k].Txs)))
    assert(manifests[k].VMStateHash == MerkleRoot(vmStates[k].Chunks))
    assert(manifests[k].LastManifestSwarmHash == SwarmHash(pack(manifests[k - 1])))

    assert(bytes.Equal(swarm[SwarmHash(pack(manifests[k]))], pack(manifests[k])))
    assert(bytes.Equal(swarm[SwarmHash(pack(blocks[k].Txs))], pack(blocks[k].Txs)))
}
