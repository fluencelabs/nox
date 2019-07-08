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

import cats.Id
import cats.data.{NonEmptyList, Validated, ValidatedNel}
import cats.effect.Sync
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.compose._
import cats.syntax.profunctor._
import fluence.codec.PureCodec
import fluence.codec.bits.BitsCodecs
import fluence.crypto.Crypto
import fluence.kad.Kademlia
import fluence.kad.protocol.{Key, Node}
import fluence.log.{Log, LogFactory}
import io.circe.{Encoder, Json}
import io.circe.syntax._
import org.http4s.dsl._
import org.http4s.headers.Authorization
import org.http4s.syntax.string._
import org.http4s.{AuthScheme, Credentials, HttpRoutes, ParseFailure, QueryParamDecoder, QueryParameterValue, Request}

import scala.language.higherKinds

class KademliaHttp[F[_]: Sync, C](
  kademlia: Kademlia[F, C],
  readNode: Crypto.Func[String, Node[C]],
  writeNode: PureCodec.Func[Node[C], String]
) {
  private implicit object KeyDecoder extends QueryParamDecoder[Key] {
    override def decode(value: QueryParameterValue): ValidatedNel[ParseFailure, Key] =
      Validated
        .fromEither(
          Key.fromB58[Id](value.value).value
        )
        .leftMap(err ⇒ NonEmptyList.one(ParseFailure(err.message, "Key codec failure")))
  }

  val FluenceAuthScheme: AuthScheme = "fluence".ci

  private val readNodeFromToken: Crypto.Func[String, Node[C]] =
    Crypto.fromOtherFunc(
      BitsCodecs.Base64.base64ToVector.direct.rmap(_.toArray).rmap(new String(_))
    )(Crypto.liftCodecErrorToCrypto) >>> readNode

  def routes()(implicit dsl: Http4sDsl[F], lf: LogFactory[F]): HttpRoutes[F] = {
    import dsl._

    object KeyQ extends QueryParamDecoderMatcher[Key]("key")
    object LookupAwayQ extends OptionalQueryParamDecoderMatcher[Key]("awayFrom")
    object NeighborsQ extends OptionalQueryParamDecoderMatcher[Int]("n")

    implicit val encodeNode: Encoder[Node[C]] = n ⇒ writeNode.runEither[Id](n).map(Json.fromString).getOrElse(Json.Null)

    import kademlia.handleRPC

    HttpRoutes
      .of[F] {
        case req @ GET -> Root / "lookup" :? KeyQ(key) +& LookupAwayQ(awayOpt) +& NeighborsQ(n) ⇒
          // Fallback to default 8 for number of neighbors to lookup
          val neighbors = n.getOrElse(8)

          LogFactory[F].init("kad-http", "lookup") >>= { implicit log: Log[F] ⇒
            updateOnReq(req) *>
              awayOpt
                .fold(handleRPC.lookup(key, neighbors))(handleRPC.lookupAway(key, _, neighbors))
                .map(_.asJson.noSpaces)
                .semiflatMap(Ok(_))
                .valueOrF(err ⇒ InternalServerError(err.toString)) // TODO render errors properly

          }

        case req @ (POST | GET) -> Root / "ping" ⇒
          LogFactory[F].init("kad-http", "ping") >>= { implicit log: Log[F] ⇒
            updateOnReq(req) >>
              handleRPC
                .ping()
                .map(_.asJson.noSpaces)
                .semiflatMap(Ok(_))
                .valueOrF(err ⇒ InternalServerError(err.toString)) // TODO render errors properly
          }
      }
  }

  /**
   * For an incoming request, fetches the Fluence auth token from a request header, parses it and updates RoutingTable.
   *
   * @param req Incoming request
   * @param log Log corresponding to current request
   * @return The same request
   */
  def updateOnReq[G[_]](
    req: Request[G]
  )(implicit log: Log[F]): F[Request[G]] =
    req.headers.get(Authorization).fold(req.pure[F]) {
      case Authorization(Credentials.Token(FluenceAuthScheme, tkn)) ⇒
        readNodeFromToken[F](tkn).value.flatMap {
          case Left(err) ⇒
            // TODO mention error in response header as well?
            log.debug(s"Auth token check failed: $err") as
              req
          case Right(node) ⇒
            // TODO check request origin?
            log.debug(s"Updating node from incoming request: $node") *>
              kademlia.update(node).as(req)
        }
      case h ⇒
        log.trace(s"Wrong authorization scheme, expected $FluenceAuthScheme, got header: " + h) as req
    }

}
