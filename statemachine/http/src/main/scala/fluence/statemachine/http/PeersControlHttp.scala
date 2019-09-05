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

package fluence.statemachine.http

import cats.effect.Sync
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.command.PeersControl
import org.http4s.dsl.Http4sDsl
import org.http4s.{DecodeResult, EntityDecoder, HttpRoutes, MalformedMessageBodyFailure}
import scodec.bits.ByteVector

import scala.language.higherKinds

object PeersControlHttp {

  def routes[F[_]: LogFactory: Sync](peersControl: PeersControl[F])(implicit dsl: Http4sDsl[F]): HttpRoutes[F] = {
    import StateMachineHttp.logReq
    import dsl._

    implicit val vkhex: EntityDecoder[F, ByteVector] =
      EntityDecoder[F, String]
        .flatMapR(
          hex ⇒
            ByteVector
              .fromHex(hex)
              .fold(DecodeResult.failure[F, ByteVector](MalformedMessageBodyFailure("Not a hex")))(
                DecodeResult.success(_)
              )
        )

    HttpRoutes.of[F] {
      case req @ POST -> Root / "drop" =>
        for {
          implicit0(log: Log[F]) ← logReq[F](req)
          validatorKey <- req.as[ByteVector]
          res <- peersControl.dropPeer(validatorKey).value
          result <- res.fold(
            err ⇒ InternalServerError(err.getMessage),
            _ ⇒ Ok()
          )
        } yield result
    }
  }

}
