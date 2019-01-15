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
  hash: String,
  nonce: String,
  blockHash: String,
  blockNumber: BigInt,
  transactionIndex: BigInt,
  from: String,
  to: String,
  value: BigInt,
  gasPrice: BigInt,
  gas: BigInt,
  input: String,
  creates: String,
  publicKey: String,
  raw: String,
  r: String,
  s: String,
  v: Long
) extends Transaction

object Transaction {

  def apply(transaction: TransactionResult[_]): Transaction =
    transaction match {
      case hash: EthBlock.TransactionHash ⇒
        TransactionHash(hash.get())

      case t: EthBlock.TransactionObject ⇒
        TransactionObject(
          t.getHash,
          t.getNonceRaw,
          t.getBlockHash,
          t.getBlockNumber,
          t.getTransactionIndex,
          t.getFrom,
          t.getTo,
          t.getValue,
          t.getGasPrice,
          t.getGas,
          t.getInput,
          t.getCreates,
          t.getPublicKey,
          t.getRaw,
          t.getR,
          t.getS,
          t.getV
        )

    }
}
