package protocol

type Transaction struct {
  Invoke []byte // function name & arguments + required metadata
  Seal   Seal   // client signature
}

type Transactions = []Transaction

// deserializes a byte array into the transaction
func TransactionUnpack([]byte) Transaction { panic("") }

// deserializes a byte array into the transactions list
func TransactionsUnpack([]byte) Transactions { panic("") }
