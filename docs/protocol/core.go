package protocol

type Digest     = [32]byte
type PublicKey  = [32]byte
type PrivateKey = [64]byte
type Signature  = [64]byte

type Chunk = []byte

type HashFunc = func([]byte) Digest

type MerkleProof struct {
  Path     []int      // path from the Merkle tree root to the selected chunk
  Siblings [][]Digest // Merkle tree layer –> sibling index in the layer –> sibling (chunk hash)
}

type Seal struct {
  PublicKey PublicKey
  Signature Signature
}

// computes a cryptographic hash of the input data
func Hash(data []byte) Digest { panic("") }

// produces a digital signature for the input data digest
func Sign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }

// verifies that the input data digest is signed correctly
func Verify(seal Seal, digest Digest) bool { panic("") }

// computes a Merkle root using supplied chunks as leaf data blocks in the Merkle tree
func MerkleRoot(allChunks []Chunk) Digest { panic("") }

// generates a Merkle proof for the chunk selected from the chunks list
func CreateMerkleProof(chunks []Chunk, index int32) MerkleProof { panic("") }

// verifies that the Merkle proof of the selected chunk conforms to the Merkle root
func VerifyMerkleProof(selectedChunk Chunk, proof MerkleProof, root Digest) bool { panic("") }
