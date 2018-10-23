package protocol

func TendermintBlockAppHashExample() {
  // data
  var blocks    []Block            // Tendermint blockchain
  var manifests []Manifest         // manifests

  // rules
  var k int                        // some block number

  // ∀ k:
    assertEq(blocks[k + 1].Header.AppHash, Hash(pack(manifests[k])))
}
