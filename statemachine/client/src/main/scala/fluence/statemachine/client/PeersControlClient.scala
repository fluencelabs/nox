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
import cats.syntax.functor._
import com.softwaremill.sttp.{sttp, _}
import fluence.effects.EffectError
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import fluence.log.Log
import fluence.statemachine.api.command.PeersControl
import scodec.bits.ByteVector

import scala.language.higherKinds

class PeersControlClient[F[_]: Monad: SttpEffect](host: String, port: Short) extends PeersControl[F] {

  override def dropPeer(validatorKey: ByteVector)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    sttp
      .post(uri"http://$host:$port/peers/drop")
      .body(validatorKey.toHex)
      .send()
      .toBody
      .void
      .leftMap(identity[EffectError])
}
