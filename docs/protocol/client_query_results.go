package protocol

type QueryResults struct {
  Chunks           map[int]Chunk    // selected virtual machine state chunks
  ChunksProofs     []MerkleProof    // Merkle proofs: chunks belong to the virtual machine state
  Manifests        [3]Manifest      // block manifests
}

func ClientQueryResultsExample() {
  // data
  var vmStates       []VMState      // virtual machine states
  var manifests      []Manifest     // manifests for blocks stored in Swarm

  // rules
  var k       int                   // some block number
  var t       int                   // some virtual machine state chunk number
  var p       int                   // some manifest index

  var results QueryResults          // results returned for the block `k`

  // ∀ k:
  // ∀ t ∈ range results.Chunks:
  assertEq(results.Chunks[t], vmStates[k + 1].Chunks[t])
  assertEq(results.ChunksProofs[t], CreateMerkleProof(t, results.Chunks[t], vmStates[k + 1].Chunks))

  // ∀ p ∈ [0, 3):
  assertEq(results.Manifests[p], manifests[k + p])
}
