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

package fluence.ethclient.data

import java.math.BigInteger

import org.web3j.protocol.core.methods.response.EthBlock

case class Block(
  number: BigInt,
  hash: String,
  parentHash: String,
  nonce: String,
  sha3Uncles: String,
  logsBloom: String,
  transactionsRoot: String,
  stateRoot: String,
  receiptsRoot: String,
  author: String,
  miner: String,
  mixHash: String,
  difficulty: BigInt,
  totalDifficulty: BigInt,
  extraData: String,
  size: BigInt,
  gasLimit: BigInt,
  gasUsed: BigInt,
  timestamp: BigInt,
  transactions: Seq[Transaction],
  uncles: Seq[String],
  sealFields: Seq[String]
)

object Block {

  // converts field that can be possibly `null`
  private def convertBigInteger(bi: BigInteger) = Option(bi).map(BigInt(_)).getOrElse(BigInt(0))
  private def convertString(s: String) = Option(s).getOrElse("")

  def apply(block: EthBlock.Block): Block = {
    import block._

    import scala.collection.convert.ImplicitConversionsToScala._
    // every field in EthBlock.Block could be `null`
    new Block(
      convertBigInteger(getNumber),
      convertString(getHash),
      convertString(getParentHash),
      convertString(getNonceRaw), // null for kovan
      convertString(getSha3Uncles),
      convertString(getLogsBloom),
      convertString(getTransactionsRoot),
      convertString(getStateRoot),
      convertString(getReceiptsRoot),
      convertString(getAuthor), // empty for ganache
      convertString(getMiner),
      convertString(getMixHash),
      convertBigInteger(getDifficulty),
      convertBigInteger(getTotalDifficulty),
      convertString(getExtraData),
      convertBigInteger(getSize),
      convertBigInteger(getGasLimit),
      convertBigInteger(getGasUsed),
      convertBigInteger(getTimestamp),
      Option(getTransactions).map(_.toSeq.map(Transaction.apply)).getOrElse(Nil),
      Option(getUncles.toSeq).getOrElse(Nil),
      Option(getSealFields.toSeq).getOrElse(Nil) // null on ganache
    )
  }
}
