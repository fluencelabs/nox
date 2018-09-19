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

package fluence.ethclient.helpers
import cats.effect.{Async, Sync}
import cats.syntax.flatMap._
import fluence.ethclient.helpers.JavaFutureConversion._
import org.web3j.protocol.core.RemoteCall
import org.web3j.tx.Contract

import scala.collection.JavaConverters._
import scala.language.higherKinds

object RemoteCallOps {
  implicit class RichRemoteCall[T](call: RemoteCall[T]) {

    /**
     * Call sendAsync on RemoteCall and delay it's side effects
     * @tparam F effect
     * @return Result of the call
     */
    def call[F[_]: Async]: F[T] = {
      Async[F].delay(call).flatMap(_.sendAsync().asAsync[F])
    }
  }

  implicit class RichContract[T <: Contract](contract: T) {

    /**
     * Execute callback on the contract and convert result to scala List
     * expected usage is to retrieve events from contract
     * @param get Callback
     * @tparam F Effect
     * @tparam E Type of the event
     * @return
     */
    def getEvent[F[_]: Sync, E](get: T => java.util.List[E]): F[List[E]] = Sync[F].delay {
      get(contract).asScala.toList
    }
  }
}
