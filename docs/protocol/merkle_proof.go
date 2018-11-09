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

// splits `data` in default-sized chunks and calculates Merkle root out of them
func MerkleHash(data []byte) Digest { panic("") }

// builds Range Merkle Proof for the range of chunks
func BuildMerkleProof(chunks []Chunk, from int32, to int32) MerkleProof { panic("") }

// splits `data` in default-sized chunks and calculates proof for the specified range
func CreateMerkleProof(data []byte, offset int32, length int32) MerkleProof { panic("") }

// checks merkle proof for the range of default-sized chunks
func VerifyMerkleProof(data []byte, proof MerkleProof, merkleRoot Digest) bool { panic("") }

func VerifySwarmProof(data []byte, proof MerkleProof, merkleRoot Digest) bool { panic("") }