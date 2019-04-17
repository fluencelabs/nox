package fluence.effects.tendermint.block

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
case class Block(header: BlockHeader, txs: List[_], lastCommit: Commit) {
  type Parts = List[_]
  type Hash = String

  // SimpleHash, go: SimpleHashFromByteSlices
  // https://github.com/tendermint/tendermint/wiki/Merkle-Trees#simple-tree-with-dictionaries
  // MerkleRoot of all the fields in the header (ie. MerkleRoot(header))
  // Note:
  //    We will abuse notion and invoke MerkleRoot with arguments of type struct or type []struct.
  //    For struct arguments, we compute a [][]byte containing the amino encoding of each field in the
  //    struct, in the same order the fields appear in the struct. For []struct arguments, we compute a
  //    [][]byte by amino encoding the individual struct elements.
  def hash: Hash = ???

  // used for secure gossipping of the block during consensus
  def parts: Parts = ???
  // MerkleRoot of the complete serialized block cut into parts (ie. MerkleRoot(MakeParts(block))
  // go: SimpleProofsFromByteSlices
  def partsHash: Hash = ???

}
