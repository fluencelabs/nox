package protocol

// verifies a BFT consensus was reached on the manifest, returns nodes signed it
func VerifyVMStateConsensus(flnContract FlnContract, manifests [3]Manifest) []PublicKey {
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
  var requiredNodes = float64(2/3) * float64(len(flnContract.NodesCollaterals))
  assertTrue(signedNodes > requiredNodes)

  // checking each Tendermint node signature validity
  for _, seal := range manifests[2].LastCommit {
    VerifyTendermintSignature(flnContract, seal, manifests[2].Header.LastBlockHash)
  }

  var signedNodesKeys = make([]PublicKey, 0, len(lastCommitPublicKeys))
  for k := range lastCommitPublicKeys {
    signedNodesKeys = append(signedNodesKeys, k)
  }
  return signedNodesKeys
}
