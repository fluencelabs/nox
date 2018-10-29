package protocol

// verifies that transaction was originated by the client with enough funds deposited
func VerifyTransaction(flnContract *BasicFluenceContract, tx *Transaction, minCollateral int64){
  // checking that the client actually exists in the contract
  collateral, ok := flnContract.ClientDeposits[tx.Seal.PublicKey]
  assertTrue(ok)

  // checking that the client has enough funds
  assertTrue(collateral >= minCollateral)

  // checking that the transaction is signed by this client
  assertTrue(Verify(tx.Seal, Hash(tx.Invoke)))
}
