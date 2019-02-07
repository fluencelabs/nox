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

  def apply(block: EthBlock.Block): Block = {
    import block._

    import scala.collection.convert.ImplicitConversionsToScala._

    new Block(
      Option(getNumber).map(BigInt(_)).getOrElse(BigInt(0)), // null on Kovan with lightclient
      getHash,
      getParentHash,
      Option(getNonceRaw).getOrElse(""), // null for kovan
      getSha3Uncles,
      getLogsBloom,
      getTransactionsRoot,
      getStateRoot,
      getReceiptsRoot,
      Option(getAuthor).getOrElse(""), // empty for ganache
      getMiner,
      getMixHash,
      getDifficulty,
      getTotalDifficulty,
      getExtraData,
      getSize,
      getGasLimit,
      getGasUsed,
      getTimestamp,
      getTransactions.toSeq.map(Transaction.apply),
      getUncles,
      Option[Seq[String]](getSealFields).getOrElse(Nil) // null on ganache
    )
  }
}
