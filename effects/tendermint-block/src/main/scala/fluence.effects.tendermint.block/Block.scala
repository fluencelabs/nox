package fluence.effects.tendermint.block

import jdk.internal.org.objectweb.asm.ByteVector
import proto3.block.Header

// https://tendermint.com/docs/spec/blockchain/blockchain.html#blockid
case class BlockId(hash: String, partsCount: Int, partsHash: String)

case class BlockHeader(
                        version: String,
                        chain_id: String,
                        height: String,
                        time: String,
                        num_txs: String,
                        total_txs: String,

                        // id of the previous block
                        last_block_id: BlockId,
                        // commit from the previous block
                        last_commit_hash: String,
                        // MerkleRoot of transaction hashes
                        data_hash: String,

                        // === hashes from the app output from the prev block ===
                        // hash(list of this block's validators)
                        validators_hash: String,
                        // hash(list of next block's validators)
                        next_validators_hash: String,
                        // hash(consensus parameters)
                        consensus_hash: String,
                        // hash(app state after txs from previous block)
                        app_hash: String,
                        // root hash of all results from the txs from the previous block
                        last_results_hash: String,
                        evidence_hash: String,
                        proposer_address: String,
                      )

// TODO: to/from JSON
case class Block(header: Header, txs: List[_]) {
  type Parts = List[ByteVector]
  type Hash = ByteVector
  type Tx = ByteVector
  type Evidence = ByteVector
  type Precommits = List[proto3.Vote] // also Vote = CommitSig in Go

  // SimpleHash, go: SimpleHashFromByteSlices
  // https://github.com/tendermint/tendermint/wiki/Merkle-Trees#simple-tree-with-dictionaries
  // MerkleRoot of all the fields in the header (ie. MerkleRoot(header))
  // Note:
  //    We will abuse notion and invoke MerkleRoot with arguments of type struct or type []struct.
  //    For struct arguments, we compute a [][]byte containing the amino encoding of each field in the
  //    struct, in the same order the fields appear in the struct. For []struct arguments, we compute a
  //    [][]byte by amino encoding the individual struct elements.
  def blockHash(): Hash = {
    fillHeader()
    headerHash()
  }

  // used for secure gossipping of the block during consensus
  def parts(): Parts = ???
  // MerkleRoot of the complete serialized block cut into parts (ie. MerkleRoot(MakeParts(block))
  // go: SimpleProofsFromByteSlices
  def partsHash(): Hash = ???

  // Calculates 3 hashes, should be called before blockHash()
  def fillHeader(): Unit = ???
  /*
		b.LastCommitHash = b.LastCommit.Hash() // commitHash
		b.DataHash = b.Data.Hash() // txsHash
		b.EvidenceHash = b.Evidence.Hash()
  */

  def headerHash(): ByteVector = {
    fillHeader() // don't forget it's already called in blockHash (meh)
    val data = List (
      Amino.encode(header.version),
      Amino.encode(header.chainID),
      Amino.encode(header.height),
      Amino.encode(header.time),
      Amino.encode(header.numTxs),
      Amino.encode(header.totalTxs),
      Amino.encode(header.lastBlockID),
      Amino.encode(header.lastCommitHash),
      Amino.encode(header.dataHash),
      Amino.encode(header.validatorsHash),
      Amino.encode(header.nextValidatorsHash),
      Amino.encode(header.consensusHash),
      Amino.encode(header.appHash),
      Amino.encode(header.lastResultsHash),
      Amino.encode(header.evidenceHash),
      Amino.encode(header.proposerAddress),
    )

    Merkle.simpleHash(data)
  }

  // Merkle hash of all precommits (some of them could be null?)
  def commitHash(precommits: List[proto3.Vote]) = ???
  /*
    for i, precommit := range commit.Precommits {
			bs[i] = cdcEncode(precommit)
		}
		commit.hash = merkle.SimpleHashFromByteSlices(bs)
  */

  // Merkle hash from the list of TXs
  def txsHash(txs: List[Tx]) = Merkle.simpleHash(txs)
  /*
    for i := 0; i < len(txs); i++ {
      txBzs[i] = txs[i].Hash()
    }
    return merkle.SimpleHashFromByteSlices(txBzs)
  */

  // Hash of the single tx, go: tmhash.Sum(tx) -> SHA256.sum
  def singleTxHash(tx: Tx) = SHA256.sum(tx)

  def evidenceHash(evl: List[Evidence]) = Merkle.simpleHash(evl)
  /*
    for i := 0; i < len(evl); i++ {
      evidenceBzs[i] = evl[i].Bytes()
    }
    return merkle.SimpleHashFromByteSlices(evidenceBzs)
  */

}

object Merkle {
  def simpleHash(data: List[ByteVector]): ByteVector = ???
}

object SHA256 {
  def sum(bs: ByteVector): ByteVector = ???
}

object Amino {
  def encode(any: Any): ByteVector = ???
}
