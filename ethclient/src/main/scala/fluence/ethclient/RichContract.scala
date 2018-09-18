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

package fluence.ethclient
import cats.effect.{Async, Sync}
import org.web3j.protocol.core.RemoteCall
import org.web3j.protocol.core.methods.response.TransactionReceipt
import org.web3j.tx.Contract

import scala.collection.JavaConverters._
import scala.language.higherKinds

/**
 * Wrapper for monadic calls on web3j contract
 *
 * @param contract wrapped contract
 * @tparam T type of the contract
 */
case class RichContract[T <: Contract](contract: T) {

  /**
   * @param call Function for executing contract methods
   * @tparam F Effect
   */
  def call[F[_]: Async](call: T => RemoteCall[TransactionReceipt]): F[TransactionReceipt] = {
    call(contract).sendAsync().asAsync[F]
  }

  def getEvent[F[_]: Sync, E](get: T => java.util.List[E]): F[List[E]] = Sync[F].delay {
    get(contract).asScala.toList
  }
}
