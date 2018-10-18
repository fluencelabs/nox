package protocol

// verifies that transaction was originated by the client with enough funds deposited
func VerifyTransaction(flnContract *FlnContract, tx *Transaction, minCollateral int64){
  // checking that the client actually exists in the contract
  collateral, ok := flnContract.ClientCollaterals[tx.Seal.PublicKey]
  assert(ok)

  // checking that the client has enough funds
  assert(collateral >= minCollateral)

  // checking that the transaction is signed by this client
  assert(Verify(tx.Seal, Hash(tx.Invoke)))
}
