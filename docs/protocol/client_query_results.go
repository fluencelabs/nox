package protocol

import "bytes"
import "reflect"

type QueryResults struct {
  Chunks           map[int]Chunk    // selected virtual machine state chunks
  ChunksProofs     []MerkleProof    // Merkle proofs: chunks belong to the virtual machine state
  Manifests        [3]Manifest      // block manifests
  ManifestReceipts [3]SwarmReceipt  // Swarm receipts for block manifests
  TxsReceipt       SwarmReceipt     // Swarm receipt for block transactions
}

func ClientQueryResultsExample() {
  // data
  var blocks         []Block        // Tendermint blockchain
  var vmStates       []VMState      // virtual machine states
  var manifests      []Manifest     // manifests for blocks stored in Swarm

  // rules
  var k       int                   // some block number
  var t       int                   // some virtual machine state chunk number
  var p       int                   // some manifest index

  var results QueryResults          // results returned for the block `k`

  // ∀ k:
    // ∀ t ∈ range results.Chunks:
      assert(bytes.Equal(results.Chunks[t], vmStates[k + 1].Chunks[t]))
      assert(reflect.DeepEqual(results.ChunksProofs[t], CreateMerkleProof(t, results.Chunks[t], vmStates[k + 1].Chunks)))

    // ∀ p ∈ [0, 3):
      assert(reflect.DeepEqual(results.Manifests[p], manifests[k + p]))
      assert(results.ManifestReceipts[p] == SwarmUpload(pack(results.Manifests[p])))

      assert(results.TxsReceipt == SwarmUpload(pack(blocks[k].Txs)))
}
