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

package fluence.kad.http.dht

import cats.effect.Sync
import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.functor._
import fluence.kad.dht.{DhtRpc, DhtValueNotFound}
import fluence.kad.http.KeyHttp
import fluence.log.LogFactory
import io.circe.{Decoder, Encoder}
import io.circe.syntax._
import io.circe.parser.parse
import org.http4s.HttpRoutes
import org.http4s.dsl.Http4sDsl

import scala.language.higherKinds

/**
 * DHT server implementation.
 *
 * @param prefix URL prefix for routes
 */
abstract class DhtHttp[F[_]](val prefix: String) {
  def routes()(implicit dsl: Http4sDsl[F], lf: LogFactory[F]): HttpRoutes[F]
}

object DhtHttp {

  /**
   * Builds a new DhtHttp instance
   *
   * @param prefix URL prefix
   * @param local Probably an instance of [[fluence.kad.dht.DhtLocalStore]]
   * @tparam F Effect
   * @tparam V Value
   */
  def apply[F[_]: Sync, V: Encoder: Decoder](
    prefix: String,
    local: DhtRpc[F, V]
  ): DhtHttp[F] =
    new DhtHttp[F](prefix) {
      override def routes()(implicit dsl: Http4sDsl[F], lf: LogFactory[F]): HttpRoutes[F] = {
        import dsl._
        import KeyHttp._

        HttpRoutes.of[F] {
          case GET -> Root / KeyVar(key) ⇒
            LogFactory[F].init("dht-http" -> s"$prefix/get", "key" -> key.asBase58) >>= { implicit log ⇒
              local.retrieve(key).value.flatMap {
                case Left(DhtValueNotFound(_)) ⇒
                  NotFound()

                case Left(err) ⇒
                  log.error("Getting data errored", err) *>
                    InternalServerError(err.getMessage)

                case Right(value) ⇒
                  Ok(value.asJson.noSpaces)
              }
            }

          case req @ PUT -> Root / KeyVar(key) ⇒
            LogFactory[F].init("dht-http" -> s"$prefix/put", "key" -> key.asBase58) >>= { implicit log ⇒
              req.as[String].map(parse).map(_.flatMap(_.as[V])).flatMap {
                case Left(pf) ⇒
                  BadRequest(s"Cannot parse input: ${pf.getMessage}")
                case Right(v) ⇒
                  local.store(key, v).value.flatMap {
                    case Left(err) ⇒
                      log.error("Storing data errored", err) *>
                        InternalServerError(err.getMessage)
                    case Right(_) ⇒
                      NoContent()
                  }
              }

            }
        }
      }
    }

}
