package protocol

import (
  "bytes"
  "crypto/sha256"
  "encoding/hex"
  "fmt"
  "math"
)

// Merkle Tree representation
type MerkleTree struct {
  Root   Node
  Height uint32
}

func (t MerkleTree) String() string {
  var description bytes.Buffer

  description.WriteString("Tree:\n")
  var children = []Node{t.Root}
  var nextChildren []Node
  var level = 1
  for {
    for _, c := range children {
      description.WriteString(hex.EncodeToString(c.Hash[:4]))
      description.WriteString("  ")

      nextChildren = append(nextChildren[:], c.Children...)
    }
    description.WriteString("\n")

    if len(nextChildren) == 0 {
      break
    }

    level++
    children = nextChildren
    nextChildren = nil
  }

  return description.String()
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

func (p RangeMerkleProof) String() string {
  var description bytes.Buffer

  description.WriteString("Proof:\n  Chunks:")
  for _, c := range p.Chunks {
    description.WriteString(" ")
    description.WriteString(hex.EncodeToString(c))
  }
  description.WriteString("\n")
  description.WriteString("  Hashes:")

  for _, h := range p.Hashes {
    description.WriteString("\n    ")
    description.WriteString(hex.EncodeToString(h[0][:4]))
    description.WriteString(" | ")
    description.WriteString(hex.EncodeToString(h[1][:4]))
  }
  description.WriteString("\n")

  return description.String()
}

type ByteRange struct {
  Offset    int32
  Length    int32
  ChunkSize int32
}

// Fluence Merkle Tree chunk size
const FlChunkSize int32 = 4

// Swarm Merkle Tree chunk size
const SwChunkSize int32 = 4000

func Max(a int32, b int32) int32 {
  if a > b {
    return a
  }

  return b
}

func Min(a int32, b int32) int32 {
  if a < b {
    return a
  }

  return b
}

// split `data` in chunks of size `chunk`
func Split(data []byte, chunk int32) []Chunk {
  var dataLen = int32(len(data))
  var chunksLen = dataLen / chunk
  if (dataLen % chunk) != int32(0) {
    chunksLen++
  }

  var result = make([]Chunk, chunksLen)
  for i := int32(0); i < chunksLen; i++ {
    var start = i * chunk
    var end = Min((i+1)*chunk, dataLen)
    result[i] = data[start:end]
  }

  return result
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

func nextPower2(n int32) (int32, uint32) {
  var log = math.Log2(float64(n))
  var pow = math.Floor(log)
  if math.Remainder(log, 1.0) != 0.0 {
    pow++
  }

  return int32(math.Pow(2, pow)), uint32(pow)
}

func hash(data []byte) Digest {
  var result = sha256.Sum256(data)
  return result
}

// builds a Merkle Tree out of chunks applying hashFn as a hash function
func BuildMerkleTree(chunks []Chunk, hashFn HashFunc) MerkleTree {
  var chunksLen = int32(len(chunks))
  var nodesCount, power = nextPower2(chunksLen)
  var bottom = make([]Node, nodesCount)

  var i int32
  for ; i < chunksLen; i++ {
    bottom[i] = Node{
      Hash: hash(chunks[i]),
    }
  }

  var empty = hash(make([]byte, len(chunks[0])))
  for ; i < nodesCount; i++ {
    bottom[i] = Node{
      Hash: empty,
    }
  }

  var tree = make([]Node, nodesCount/2)
  for len(tree) != 0 {
    for i = 0; i < int32(len(bottom))-1; i += 2 {
      var h = hash(append(bottom[i].Hash[:], bottom[i+1].Hash[:]...))
      var n = Node{
        Hash:     h,
        Children: []Node{bottom[i], bottom[i+1]},
      }
      bottom[i].Parent = &n
      bottom[i+1].Parent = &n
      tree[i/2] = n
    }
    bottom = tree
    tree = make([]Node, len(tree)/2)
  }

  return MerkleTree{
    Root:   bottom[0],
    Height: power + 1, // power is calculated just from the number of bottom tree nodes, so +1 is mandatory
  }
}

// returns Merkle Tree nodes that form a Range Merkle Proof for chunks in the interval [start, stop]
func RangeProof(tree MerkleTree, start int32, stop int32) [][2]Digest {
  // -2 comes from: -1 since we already have 0x01, and -1 since we're starting at the level 1
  var mask int32 = 0x1 << (tree.Height - 2)

  // the resulting proof
  // first [] stands for the level of the tree, starting at 1. So proof[0] contains hashes for the level 1
  // the second [] stands for the left or right branch of proof. So proof[0][0] contains leftmost needed hash at the level 1.
  var proof = make([][2]Digest, (tree.Height - 1))

  var nextLeft = tree.Root
  var nextRight = tree.Root

  // initially level = 0 but since we don't need hashes of the root, algorithm actually starts on the level one
  // we don't need a hash of the root in the proof because it's computable from children on level 1
  for level := uint32(0); level < (tree.Height - 1); {
    if mask&start != 0 {
      proof[level][0] = nextLeft.Children[0].Hash
    }

    if mask&stop == 0 {
      proof[level][1] = nextRight.Children[1].Hash
    }

    if mask&start == 0 {
      nextLeft = nextLeft.Children[0]
    } else {
      nextLeft = nextLeft.Children[1]
    }

    if mask&stop == 0 {
      nextRight = nextRight.Children[0]
    } else {
      nextRight = nextRight.Children[1]
    }

    level++
    mask = mask >> 1
  }

  return proof
}

// builds Merkle Proof for a range of `length` bytes starting at `offset` by splitting them in chunks of size `chunk`
func BuildRangeMerkleProof(data []byte, byteRange ByteRange, hashFn HashFunc) RangeMerkleProof {
  var chunks = Split(data, byteRange.ChunkSize)
  var startChunk = byteRange.StartChunk()
  var stopChunk = byteRange.StopChunk()

  var bottomChunks = chunks[startChunk : stopChunk+1]

  var tree = BuildMerkleTree(chunks, hashFn)
  fmt.Printf("%v\n", tree)

  var proof = RangeProof(tree, startChunk, stopChunk)

  return RangeMerkleProof{
    Chunks:     bottomChunks,
    Hashes:     proof,
    StartChunk: startChunk,
    StopChunk:  stopChunk,
  }
}

func CheckProof(root Digest, proof RangeMerkleProof) bool {
  var bottom = make([]Digest, len(proof.Chunks))
  for i := 0; i < len(bottom); i++ {
    bottom[i] = hash(proof.Chunks[i])
  }
  var height = len(proof.Hashes)
  var mask int32 = 0x1

  var tree []Digest
  for level := height - 1; level >= 0; level-- {
    if (proof.StartChunk & mask) != 0 {
      bottom = append([]Digest{proof.Hashes[level][0]}, bottom...)
    }

    if (proof.StopChunk & mask) == 0 {
      bottom = append(bottom, proof.Hashes[level][1])
    }

    tree = make([]Digest, len(bottom)/2)

    for i := 0; i < len(bottom)-1; i += 2 {
      tree[i/2] = hash(append(bottom[i][:], bottom[i+1][:]...))
    }

    bottom = tree
    mask = mask << 1
  }

  fmt.Println("tree[0]\t", len(tree[0]), tree[0])
  fmt.Println("root\t", len(root), root)
  return tree[0] == root
}

func printArray(label string, arr [][32]byte) {
  fmt.Printf("%s: ", label)
  for _, c := range arr {
    fmt.Printf("%v ", hex.EncodeToString(c[:4]))
  }
  fmt.Println()
}

func (contract ValidationFluenceContract) SubmitProofs(flProof RangeMerkleProof, swProof RangeMerkleProof) bool {
  panic("")
}
