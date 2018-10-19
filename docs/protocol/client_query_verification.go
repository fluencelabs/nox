package protocol

func VerifyResultsManifestsStorage(swarmContract *SwarmContract, results *QueryResults, minCollateral int64) {
  for p := 0; p < 3; p++ {
    // checking that the receipt is issued for the correct manifest
    assertEq(results.ManifestReceipts[p].ContentHash, SwarmHash(pack(results.Manifests[p])))

    // checking that the swarm node has enough funds
    var swarmNodeId = results.ManifestReceipts[p].Insurance.PublicKey
    assertTrue(swarmContract.Collaterals[swarmNodeId] >= minCollateral)

    // checking that the receipt is signed by this swarm node
    assertTrue(SwarmVerify(results.ManifestReceipts[p].Insurance, results.ManifestReceipts[p].ContentHash))
  }
}

func VerifyResultsSwarmConnectivity(results *QueryResults) {
  for p := 0; p < 2; p++ {
    assertEq(results.Manifests[p + 1].LastManifestSwarmHash, SwarmHash(pack(results.Manifests[p])))
  }
}

func VerifyResultsAppStateConnectivity(results *QueryResults) {
  for p := 0; p < 2; p++ {
    assertEq(results.Manifests[p + 1].Header.AppHash, Hash(pack(results.Manifests[p])))
  }
}

func VerifyResultsBlocks(flnContract *FlnContract, results *QueryResults, minCollateral int64) {
  for p := 0; p < 2; p++ {
    // checking that BFT consensus was actually reached
    var signedNodes = float64(len(results.Manifests[p + 1].LastCommit))
    var requiredNodes = float64(2/3) * float64(len(flnContract.NodesCollaterals))
    assertTrue(signedNodes > requiredNodes)

    for _, signature := range results.Manifests[p + 1].LastCommit {
      // checking that the real-time node has enough funds
      var tmNodeId = signature.PublicKey
      assertTrue(flnContract.NodesCollaterals[tmNodeId] >= minCollateral)

      // checking that the block commit is signed by this node
      assertTrue(TmVerify(signature, TmMerkleRoot(packMulti(results.Manifests[p].Header))))
    }
  }
}

func VerifyResultsChunks(results QueryResults) {
  for t := range results.Chunks {
    assertTrue(VerifyMerkleProof(results.Chunks[t], &results.ChunksProofs[t], results.Manifests[1].VMStateHash))
  }
}
