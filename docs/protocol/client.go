package protocol

func VerifySwarmReceipt(contract SwarmContract, receipt SwarmReceipt) {
  var minDeposit int64 = 1000000

  // checking that the Swarm node has enough funds
  var swarmNodeId = receipt.Insurance.PublicKey
  assertTrue(contract.Deposits[swarmNodeId] >= minDeposit)

  // checking that the receipt is signed by this Swarm node
  assertTrue(SwarmVerify(receipt.Insurance, receipt.ContentHash))
}

func VerifyTendermintSignature(contract BasicFluenceContract, seal Seal, blockHash Digest) {
  var minDeposit int64 = 1000000

  // checking that the Tendermint node has enough funds
  var tmNodeId = seal.PublicKey
  assertTrue(contract.NodesDeposits[tmNodeId] >= minDeposit)

  // checking that the receipt is signed by this Tendermint node
  assertTrue(TmVerify(seal, blockHash))
}

func VerifyManifestsReceipts(contract SwarmContract, response QueryResponse) {
  // checking that manifests and transactions receipts are properly signed by Swarm nodes
  for _, manifest := range response.Manifests {
    VerifySwarmReceipt(contract, manifest.LastManifestReceipt)
    VerifySwarmReceipt(contract, manifest.TxsReceipt)
  }

  // checking that each manifest points correctly to the previous manifest via the Swarm receipt
  for i := 0; i < 2; i++ {
    var manifest = response.Manifests[i+1]
    var prevManifest = response.Manifests[i]

    assertEq(manifest.LastManifestReceipt.ContentHash, SwarmHash(pack(prevManifest)))
  }
}

// verifies a BFT consensus was reached on the manifest, returns nodes signed it
func VerifyVMStateConsensus(contract BasicFluenceContract, manifests [3]Manifest) []PublicKey {
  // checking connection between the VM state in the manifest 0 and Tendermint signatures in the manifest 2
  assertEq(manifests[1].Header.AppHash, Hash(pack(manifests[0])))
  assertEq(manifests[2].Header.LastBlockHash, TmMerkleRoot(packMulti(manifests[1].Header)))

  // counting the number of unique Tendermint nodes public keys
  var lastCommitPublicKeys = make(map[PublicKey]bool)
  for _, seal := range manifests[2].LastCommit {
    lastCommitPublicKeys[seal.PublicKey] = true
  }

  // checking that BFT consensus was actually reached
  var signedNodes = float64(len(lastCommitPublicKeys))
  var requiredNodes = float64(2/3) * float64(len(contract.NodesDeposits))
  assertTrue(signedNodes > requiredNodes)

  // checking each Tendermint node signature validity
  for _, seal := range manifests[2].LastCommit {
    VerifyTendermintSignature(contract, seal, manifests[2].Header.LastBlockHash)
  }

  return keys(lastCommitPublicKeys).([]PublicKey)
}

func VerifyResponseChunks(results QueryResponse) {
  for k := range results.Chunks {
    assertTrue(VerifyMerkleProof(results.Chunks[k], results.Proofs[k], results.Manifests[0].VMStateHash))
  }
}
