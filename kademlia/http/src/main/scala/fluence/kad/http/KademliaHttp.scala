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

package fluence.kad.http

import cats.{Id, Monad}
import io.circe.syntax._
import io.circe.Encoder
import io.circe.Json
import cats.data.{NonEmptyList, Validated, ValidatedNel}
import cats.effect.{LiftIO, Sync}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import cats.syntax.functor._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.codec.PureCodec
import org.http4s.{
  AuthScheme,
  Credentials,
  Http,
  HttpRoutes,
  ParseFailure,
  QueryParamDecoder,
  QueryParameterValue,
  Request
}
import org.http4s.dsl._
import org.http4s.headers.Authorization
import org.http4s.syntax.string._

import scala.language.higherKinds

object KademliaHttp {
  private implicit object KeyDecoder extends QueryParamDecoder[Key] {
    override def decode(value: QueryParameterValue): ValidatedNel[ParseFailure, Key] =
      Validated
        .fromEither(
          Key.fromB58[Id](value.value).value
        )
        .leftMap(err ⇒ NonEmptyList.one(ParseFailure(err.message, "Key codec failure")))
  }

  val FluenceAuthScheme: AuthScheme = "fluence".ci

  def routes[F[_]: Sync: LiftIO, C](
    handler: KademliaRpc[C]
  )(implicit dsl: Http4sDsl[F], writeNode: PureCodec.Func[Node[C], String]): HttpRoutes[F] = {
    import dsl._

    object KeyQ extends QueryParamDecoderMatcher[Key]("key")
    object LookupAwayQ extends OptionalQueryParamDecoderMatcher[Key]("awayFrom")
    object NeighborsQ extends OptionalQueryParamDecoderMatcher[Int]("n")

    implicit val encodeNode: Encoder[Node[C]] = n ⇒ writeNode.runEither[Id](n).map(Json.fromString).getOrElse(Json.Null)

    HttpRoutes
      .of[F] {
        case GET -> Root / "lookup" :? KeyQ(key) & LookupAwayQ(awayOpt) & NeighborsQ(n) ⇒
          // Just a magic number to omit ?n query param
          val neighbors = n.getOrElse(8)
          awayOpt
            .fold(handler.lookup(key, neighbors))(handler.lookupAway(key, _, neighbors))
            .to[F]
            .map(_.asJson.noSpaces)
            .flatMap(Ok(_))

        case POST -> Root / "ping" ⇒
          handler.ping().to[F].map(_.asJson.noSpaces).flatMap(Ok(_))
      }
  }

  def updateOnReq[F[_]: Monad, G[_], C](
    cb: Node[C] ⇒ F[Unit],
    read: PureCodec.Func[String, Node[C]],
    http: Http[F, G]
  ): Http[F, G] =
    http.compose(
      (req: Request[G]) ⇒
        req.headers.get(Authorization).fold(req.pure[F]) {
          case Authorization(Credentials.Token(FluenceAuthScheme, tkn)) ⇒
            read[F](tkn).value.flatMap {
              case Left(_) ⇒
                // TODO mention error in response header?
                req.pure[F]
              case Right(node) ⇒
                // TODO check request origin?
                cb(node).as(req)
            }
          case _ ⇒
            req.pure[F]
      }
    )

}
