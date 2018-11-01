package protocol

import (
  "math"
)

// Merkle Tree representation
type MerkleTree struct {
  Root Node
}

// node of the Merkle Tree
type Node struct {
  Hash     Digest
  Children []Node
  Parent   *Node
}

// Merkle Proof for a range of chunks
type RangeMerkleProof struct {
  Chunks     []Chunk
  Hashes     [][2]Digest
  StartChunk int32
  StopChunk  int32
}

type ByteRange struct {
  Offset    int32
  Length    int32
  ChunkSize int32
}

// Fluence Merkle Tree chunk size
const FlChunkSize int32 = 4000

// Swarm Merkle Tree chunk size
const SwChunkSize int32 = 6000

// split `data` in chunks of size `chunk`
func Split(data []byte, chunk int32) []Chunk { panic("") }

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

// Merkle Proof for an inclusion of a range of chunks in Fluence Merkle Tree
func FlRangeMerkleProof(data []byte, offset int32, length int32) RangeMerkleProof {
  var byteRange = ByteRange{
    Offset:    offset,
    Length:    length,
    ChunkSize: FlChunkSize,
  }
  return BuildRangeMerkleProof(data, byteRange, Hash)
}

// Merkle Proof for an inclusion of a range of chunks in Swarm Merkle Tree
func SwRangeMerkleProof(data []byte, offset int32, length int32) RangeMerkleProof {
  var byteRange = ByteRange{
    Offset:    offset,
    Length:    length,
    ChunkSize: SwChunkSize,
  }
  return BuildRangeMerkleProof(data, byteRange, SwarmHash)
}

// builds a Merkle Tree out of chunks applying hashFn as a hash function
func BuildMerkleTree(chunks []Chunk, hashFn HashFunc) MerkleTree { panic("") }

// returns Merkle Tree nodes that form a Range Merkle Proof for chunks in the interval [start, stop]
func RangeProof(tree MerkleTree, start int32, stop int32) [][2]Digest { panic("") }

// builds Merkle Proof for a range of `length` bytes starting at `offset` by splitting them in chunks of size `chunk`
func BuildRangeMerkleProof(data []byte, byteRange ByteRange, hashFn HashFunc) RangeMerkleProof {
  var chunks = Split(data, byteRange.ChunkSize)
  var startChunk = byteRange.StartChunk()
  var stopChunk = byteRange.StopChunk()

  var bottomChunks = chunks[startChunk : stopChunk+1]

  var tree = BuildMerkleTree(chunks, hashFn)
  var proof = RangeProof(tree, startChunk, stopChunk)

  return RangeMerkleProof{
    Chunks:     bottomChunks,
    Hashes:     proof,
    StartChunk: startChunk,
    StopChunk:  stopChunk,
  }
}

func (contract ValidationFluenceContract) SubmitProofs(flProof RangeMerkleProof, swProof RangeMerkleProof) bool {
  panic("")
}
