package protocol

type QueryResponse struct {
  Chunks    map[int]Chunk        // selected virtual machine state chunks
  Proofs    map[int]MerkleProof  // Merkle proofs: chunks belong to the virtual machine state
  Manifests [3]Manifest          // block manifests
}

// prepares the query response
func MakeQueryResponse(manifests [3]Manifest, vmState VMState, chunksIndices []int) QueryResponse {
  var chunks = make(map[int]Chunk)
  var proofs = make(map[int]MerkleProof)

  for _, index := range chunksIndices {
    var chunk = vmState.Chunks[index]

    chunks[index] = chunk
    proofs[index] = CreateMerkleProof(index, chunk, vmState.Chunks)
  }

  return QueryResponse{Chunks: chunks, Proofs: proofs, Manifests: manifests}
}
