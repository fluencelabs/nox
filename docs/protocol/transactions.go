package protocol

type Transaction struct {
  Invoke []byte  // function name & arguments + required metadata
  Seal   Seal    // client signature of the transaction
}
