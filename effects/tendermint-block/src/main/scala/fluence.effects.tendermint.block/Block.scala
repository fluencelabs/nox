/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.tendermint.block

import com.google.protobuf.ByteString
import proto3.Tendermint
import proto3.tendermint.{Header, Vote}
import scalapb_circe.Parser
import scodec.bits.ByteVector

// About BlockID: https://tendermint.com/docs/spec/blockchain/blockchain.html#blockid

// TODO: to/from JSON
case class Block(header: Header, txs: List[_]) {
  type Parts = List[ByteVector]
  type Hash = ByteVector
  type Tx = ByteVector
  type Evidence = ByteVector
  type Precommits = List[Vote] // also, Vote = CommitSig in Go

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
    val data = List(
      Amino.encode(header.version),
      Amino.encode(header.chainId),
      Amino.encode(header.height),
      Amino.encode(header.time),
      Amino.encode(header.numTxs),
      Amino.encode(header.totalTxs),
      Amino.encode(header.lastBlockId),
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
  def commitHash(precommits: List[Vote]) = ???
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

object JSON {
  val parser = new Parser(true)

  val firstVote =
    """
      |{
      |    "type": 2,
      |    "height": 16,
      |    "round": 0,
      |    "block_id": {
      |        "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |        "parts": {
      |            "total": 1,
      |            "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |        }
      |    },
      |    "timestamp": "2019-04-17T13:30:03.536359799Z",
      |    "validator_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E",
      |    "validator_index": 0,
      |    "signature": "Z09xcrfz9T6+3q1Yk+gxUo2todPI7mebKed6zO+i1pnIMPdFbSFT9JJjxo5J9HLrn4x2Fqf3QYefQ8lQGNMzBg=="
      |}
    """.stripMargin

  def vote(json: String): Vote = {
    val v = parser.fromJsonString[Vote](json)
    v.update(_.blockId.modify(id => id.withHash(fixBytes(id.hash))))
  }

  def gvote(json: String): Tendermint.Vote = {
    import com.google.protobuf.util.JsonFormat.{parser => gparser}

    val builder = Tendermint.Vote.newBuilder()
    gparser().merge(json, builder)

    val id = builder.getBlockId
    val goodHash = fixBytes(id.getHash)
    val goodId = Tendermint.BlockID.newBuilder(id).setHash(goodHash)
    builder.setBlockId(goodId)
    builder.build()
  }

  /**
   * Performs base64 encode -> hex decode -> to byte string
   *
   * Protobuf "erroneously" applied `base64 decode` to a hex string, this function fixes that
   *
   * NOTE:
   *   It's not a protobuf mistake, it's just a protocol quirk.
   *   When Tendermint encodes values to JSON to return in RPC, some bytes (i.e., common.HexBytes) are encoded in hex,
   *   while other bytes (i.e., byte[]) are encoded in base64.
   *   It's just a happy coincidence that protobuf works on that JSON at all, it wasn't meant to.
   *
   * @param bs bs = ByteString.copyFrom(base64-decode(hexString))
   * @return Good, correct bytes, that were represented by a hexString
   */
  def fixBytes(bs: ByteString): ByteString = {
    val hex = ByteVector(bs.toByteArray).toBase64
    val value = ByteVector.fromHex(hex)
    val bytes = ByteString.copyFrom(value.toArray[Byte])
    bytes
  }
}
