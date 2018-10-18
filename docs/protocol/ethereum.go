package protocol

type ExampleContract struct {
  Collaterals map[PublicKey]int64  // security deposits: node identifier –> deposit size
}

// verifies that a node has enough deposited funds
func VerifyNodeCollateral(exampleContract *ExampleContract, nodeId PublicKey, minCollateral int64) {
  assert(exampleContract.Collaterals[nodeId] >= minCollateral)
}
