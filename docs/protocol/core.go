package protocol

type Digest     = [32]byte
type PublicKey  = [32]byte
type PrivateKey = [64]byte
type Signature  = [64]byte

type HashFunc = func([]byte) Digest

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
