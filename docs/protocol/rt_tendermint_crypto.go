package protocol

// listed Tendermint functions carry the same meaning and arguments as core functions
func TmHash(data []byte) Digest { panic("") }
func TmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }
func TmVerify(seal Seal, digest Digest) bool { panic("") }
func TmMerkleRoot(allChunks []Chunk) Digest { panic("") }
