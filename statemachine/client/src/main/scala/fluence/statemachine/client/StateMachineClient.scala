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

package fluence.statemachine.client

import cats.Monad
import cats.data.EitherT
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.data.StateMachineStatus
import fluence.statemachine.api.query.QueryResponse
import com.softwaremill.sttp.{sttp, _}
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import io.circe.parser.decode
import shapeless._

import scala.language.higherKinds

/**
 * Provides access for a remote HTTP-accessible [[StateMachine]].
 */
object StateMachineClient {

  /**
   * Read only [[StateMachine]] access via HTTP
   *
   * @param host State machine's host
   * @param port State machine's port
   * @return Pure readonly [[StateMachine]] with empty command side
   */
  def readOnly[F[_]: Monad: SttpEffect](host: String, port: Short): StateMachine.Aux[F, HNil] =
    new StateMachine.ReadOnly[F] {
      override def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse] =
        sttp
          .get(uri"http://$host:$port/query?path=$path")
          .send()
          .decodeBody(decode[QueryResponse])
          .leftMap(identity[EffectError])

      override def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus] =
        sttp
          .get(uri"http://$host:$port/status")
          .send()
          .decodeBody(decode[StateMachineStatus])
          .leftMap(identity[EffectError])
    }

  /**
   * Builds a remote [[StateMachine]] access for both query (read) and command side.
   *
   * @param host State machine's host
   * @param port State machine's port
   * @return [[StateMachine]] with [[ReceiptBus]] and [[PeersControl]] command access
   */
  def apply[F[_]: Monad: SttpEffect](
    host: String,
    port: Short
  ): StateMachine.Aux[F, ReceiptBus[F] :: PeersControl[F] :: HNil] =
    readOnly[F](host, port)
      .extend[PeersControl[F]](
        new PeersControlClient[F](host, port)
      )
      .extend[ReceiptBus[F]](
        new ReceiptBusClient[F](host, port)
      )
}
