package protocol

type ExampleContract struct {
  Collaterals map[PublicKey]int64  // security deposits: node identifier –> deposit size
}

// data
var exampleContract ExampleContract  // example contract instance

// verification
func VerifyNodeCollateral(nodeId PublicKey, minCollateral int64) {
  assert(exampleContract.Collaterals[nodeId] >= minCollateral)
}
