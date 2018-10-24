package protocol

type Block struct {
  Header     Header        // block header
  LastCommit []Seal        // Tendermint nodes votes for the previous block
  Txs        Transactions  // transactions as sent by clients
}

type Header struct {
  LastBlockHash  Digest  // Merkle root of the previous block header fields
  LastCommitHash Digest  // Merkle root of the last commit votes
  TxsHash        Digest  // Merkle root of the block transactions
  AppHash        Digest  // application state hash after the previous block
}

func TendermintBlockFormationExample() {
  // data
  var blocks []Block  // Tendermint blockchain

  // rules
  var k int64         // some block number
  var i int           // some Tendermint node index

  // ∀ k:
    assertEq(blocks[k].Header.LastBlockHash, TmMerkleRoot(packMulti(blocks[k - 1].Header)))
    assertEq(blocks[k].Header.LastCommitHash, TmMerkleRoot(packMulti(blocks[k].LastCommit)))
    assertEq(blocks[k].Header.TxsHash, TmMerkleRoot(packMulti(blocks[k].Txs)))

    // ∀ i:
      assertTrue(TmVerify(blocks[k].LastCommit[i], blocks[k].Header.LastBlockHash))
}
