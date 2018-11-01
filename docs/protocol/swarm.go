package protocol

// listed Swarm functions carry the same meaning and arguments as core functions
func SwarmHash(data []byte) Digest { panic("") }
func SwarmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }
func SwarmVerify(seal Seal, digest Digest) bool { panic("") }

type SwarmContract struct {
  Deposits map[PublicKey]int64 // security deposits: node identifier –> deposit size
}

type SwarmReceipt struct {
  ContentHash Digest // Swarm hash of the stored content
  Insurance   Seal   // insurance written by the Swarm node for the accepted content
}

// uploads the content to the Swarm network, returns a receipt of responsibility
func SwarmUpload(content []byte) SwarmReceipt { panic("") }

// downloads the content from the Swarm network using the supplied receipt
func SwarmDownload(receipt SwarmReceipt) []byte { panic("") }
