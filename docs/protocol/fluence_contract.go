package protocol

type BasicFluenceContract struct {
  ClientDeposits     map[PublicKey]int64 // clients: identifier –> deposit size
  NodesDeposits      map[PublicKey]int64 // real-time nodes: identifier –> deposit size
  ValidatorsDeposits map[PublicKey]int64 // batch validators: identifier –> deposit size
}
