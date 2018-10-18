package protocol

// listed Swarm functions carry the same meaning and arguments as core functions
func SwarmHash(data []byte) Digest { panic("") }
func SwarmSign(publicKey PublicKey, privateKey PrivateKey, digest Digest) Seal { panic("") }
func SwarmVerify(seal Seal, digest Digest) bool { panic("") }

func SwarmContentExample() {
  // data
  var swarm map[Digest][]byte  // Swarm storage: hash(x) –> x

  // rules
  var content []byte           // some content

  // ∀ content:
    assertEq(swarm[SwarmHash(content)], content)
}
