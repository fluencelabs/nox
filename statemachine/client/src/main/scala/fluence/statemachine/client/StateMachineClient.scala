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
import fluence.statemachine.api.command.{HashesBus, PeersControl}
import io.circe.parser.decode
import shapeless._

import scala.language.higherKinds

object StateMachineClient {

  def readOnly[F[_]: Monad: SttpEffect](host: String, port: Short): StateMachine.Aux[F, HNil] =
    new StateMachine.ReadOnly[F] {
      // TODO implement querying!
      override def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse] =
        EitherT.leftT[F, QueryResponse](
          (throw new NotImplementedError("StateMachineClient.query, as QueryResponse format is not defined"))
            .asInstanceOf[EffectError]
        )

      override def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus] =
        sttp
          .get(uri"http://$host:$port/status")
          .send()
          .decodeBody(decode[StateMachineStatus])
          .leftMap(identity[EffectError])
    }

  def apply[F[_]: Monad: SttpEffect](
    host: String,
    port: Short
  ): StateMachine.Aux[F, HashesBus[F] :: PeersControl[F] :: HNil] =
    readOnly[F](host, port)
      .extend[PeersControl[F]](
        new PeersControlClient[F](host, port)
      )
      .extend[HashesBus[F]](
        new HashesBusClient[F](host, port)
      )
}
