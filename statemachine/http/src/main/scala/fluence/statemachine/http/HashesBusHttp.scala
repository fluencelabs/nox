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
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.log.{Log, LogFactory}
import fluence.statemachine.api.command.HashesBus
import fluence.statemachine.api.data.BlockReceipt
import org.http4s.{EntityDecoder, HttpRoutes}
import org.http4s.circe._
import org.http4s.dsl.Http4sDsl

import scala.language.higherKinds

object HashesBusHttp {

  def routes[F[_]: LogFactory: Sync](hashesBus: HashesBus[F])(implicit dsl: Http4sDsl[F]): HttpRoutes[F] = {
    import dsl._
    import StateMachineHttp.logReq

    object HeightQ extends QueryParamDecoderMatcher[Long]("height")

    implicit val bpdec: EntityDecoder[F, BlockReceipt] = jsonOf[F, BlockReceipt]

    HttpRoutes.of[F] {
      case req @ POST -> Root / "blockReceipt" =>
        for {
          implicit0(log: Log[F]) ← logReq[F](req)
          receipt <- req.as[BlockReceipt]
          res <- hashesBus.sendBlockReceipt(receipt).value
          result <- res.fold(
            err ⇒ InternalServerError(err.getMessage),
            _ ⇒ Ok()
          )
        } yield result

      case req @ GET -> Root / "vmHash" :? HeightQ(height) =>
        for {
          implicit0(log: Log[F]) ← logReq(req)
          vmHash <- hashesBus.getVmHash(height).value
          result <- vmHash.fold(
            err ⇒ InternalServerError(err.getMessage),
            h ⇒ Ok(h.toHex)
          )
        } yield result
    }
  }

}
