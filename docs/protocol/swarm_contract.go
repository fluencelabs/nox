package protocol

type SwarmContract struct {
  Collaterals map[PublicKey]int64  // security deposits: node identifier –> deposit size
}

func SwarmContractExample() {
  // data
  var swarmContract SwarmContract  // Swarm Ethereum smart contract

  {
    // just to make things compile
    _ = swarmContract
  }
}
