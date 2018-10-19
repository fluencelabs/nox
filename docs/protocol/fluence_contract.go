package protocol

type FlnContract struct {
  ClientCollaterals     map[PublicKey]int64  // clients: identifier –> deposit size
  NodesCollaterals      map[PublicKey]int64  // real-time nodes: identifier –> deposit size
  ValidatorsCollaterals map[PublicKey]int64  // batch validators: identifier –> deposit size
}

func FluenceContractExample() {
  // data
  var flnContract FlnContract  // Fluence Ethereum smart contract

  {
    // just to make things compile
    _ = flnContract
  }
}
