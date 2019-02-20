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
import org.web3j.protocol.core.methods.response.EthBlock
import org.web3j.protocol.core.methods.response.EthBlock.TransactionResult

sealed trait Transaction

case class TransactionHash(hash: String) extends Transaction

case class TransactionObject(
  hash: Option[String],
  nonce: Option[String],
  blockHash: Option[String],
  blockNumber: Option[BigInt],
  transactionIndex: Option[BigInt],
  from: Option[String],
  to: Option[String],
  value: Option[BigInt],
  gasPrice: Option[BigInt],
  gas: Option[BigInt],
  input: Option[String],
  creates: Option[String],
  publicKey: Option[String],
  raw: Option[String],
  r: Option[String],
  s: Option[String],
  v: Option[Long]
) extends Transaction

object Transaction {

  def apply(transaction: TransactionResult[_]): Transaction =
    transaction match {
      case hash: EthBlock.TransactionHash ⇒
        TransactionHash(hash.get())

      case t: EthBlock.TransactionObject ⇒
        TransactionObject(
          Option(t.getHash),
          Option(t.getNonceRaw),
          Option(t.getBlockHash),
          Option(t.getBlockNumber),
          Option(t.getTransactionIndex),
          Option(t.getFrom),
          Option(t.getTo),
          Option(t.getValue),
          Option(t.getGasPrice),
          Option(t.getGas),
          Option(t.getInput),
          Option(t.getCreates),
          Option(t.getPublicKey),
          Option(t.getRaw),
          Option(t.getR),
          Option(t.getS),
          Option(t.getV)
        )

    }
}
