package protocol

type ByteRange struct {
 Offset    int32
 Length    int32
 ChunkSize int32
}

// calculates the index of the first chunk covering byte range
func (byteRange ByteRange) StartChunk() int32 {
  return byteRange.Offset / byteRange.ChunkSize
}

// calculates the index of the last chunk covering byte range
func (byteRange ByteRange) StopChunk() int32 {
  var stop = byteRange.StartChunk() + byteRange.Length/byteRange.ChunkSize
  if (byteRange.Length % byteRange.ChunkSize) == int32(0) {
    stop--
  }

  return stop
}

type MerkleProofLayer struct {
  left  *Digest
  right *Digest
}

type RangeMerkleProof struct {
  Layers []MerkleProofLayer
}

// splits the byte sequence into chunks of specific size
func Split(data []byte, chunkSize int32) []Chunk { panic("") }

// build Range Merkle Proof for a range of chunks [startChunk, stopChunk]
func BuildMerkleRangeProof(chunks []Chunk, from int32, to int32) RangeMerkleProof { panic("") }

func (contract ValidationFluenceContract) SubmitProofs(flProof RangeMerkleProof, swProof RangeMerkleProof) bool {
 panic("")
}
