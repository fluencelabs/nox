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
  number: Option[BigInt],
  hash: Option[String],
  parentHash: Option[String],
  nonce: Option[String],
  sha3Uncles: Option[String],
  logsBloom: Option[String],
  transactionsRoot: Option[String],
  stateRoot: Option[String],
  receiptsRoot: Option[String],
  author: Option[String],
  miner: Option[String],
  mixHash: Option[String],
  difficulty: Option[BigInt],
  totalDifficulty: Option[BigInt],
  extraData: Option[String],
  size: Option[BigInt],
  gasLimit: Option[BigInt],
  gasUsed: Option[BigInt],
  timestamp: Option[BigInt],
  transactions: Seq[Transaction],
  uncles: Seq[String],
  sealFields: Seq[String]
)

object Block {

  def apply(block: EthBlock.Block): Block = {
    import block._

    import scala.collection.convert.ImplicitConversionsToScala._
    // every field in EthBlock.Block could be `null`
    new Block(
      Option(getNumber),
      Option(getHash),
      Option(getParentHash),
      Option(getNonceRaw),
      Option(getSha3Uncles),
      Option(getLogsBloom),
      Option(getTransactionsRoot),
      Option(getStateRoot),
      Option(getReceiptsRoot),
      Option(getAuthor),
      Option(getMiner),
      Option(getMixHash),
      Option(getDifficulty),
      Option(getTotalDifficulty),
      Option(getExtraData),
      Option(getSize),
      Option(getGasLimit),
      Option(getGasUsed),
      Option(getTimestamp),
      Option(getTransactions).map(_.toSeq.map(Transaction.apply)).getOrElse(Nil),
      Option(getUncles).map(_.toSeq).getOrElse(Nil),
      Option(getSealFields).map(_.toSeq).getOrElse(Nil)
    )
  }
}
