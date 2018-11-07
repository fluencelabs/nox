package protocol

type MerkleProofLayer struct {
  left  *Digest
  right *Digest
}

type MerkleProof struct {
  Layers []MerkleProofLayer
}

// splits the byte sequence into chunks of specific size
func Split(data []byte, chunkSize int32) []Chunk { panic("") }

// build Range Merkle Proof for the range of chunks
func BuildMerkleProof(chunks []Chunk, from int32, to int32) MerkleProof { panic("") }
