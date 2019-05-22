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

import cats.data.EitherT
import cats.effect.{Effect, IO}
import cats.syntax.either._
import cats.effect.syntax.effect._
import com.softwaremill.sttp._
import fluence.crypto.Crypto
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import io.circe.{Decoder, DecodingFailure}
import io.circe.parser._

import scala.language.higherKinds

class KademliaHttpClient[F[_]: Effect, C](hostname: String, port: Short, auth: String)(
  implicit s: SttpBackend[EitherT[F, Throwable, ?], Nothing],
  readNode: Crypto.Func[String, Node[C]]
) extends KademliaRpc[C] {

  // TODO: do not drop cause
  private implicit val decodeNode: Decoder[Node[C]] =
    _.as[String].flatMap(readNode.runEither[Id](_).leftMap(ce ⇒ DecodingFailure(ce.message, Nil)))

  private def call[T: Decoder](call: sttp.type ⇒ Uri ⇒ Request[String, Nothing], uri: Uri): IO[T] =
    (for {
      node <- call(sttp)(uri)
        .header(HeaderNames.Authorization, auth)
        .send()
        .map(_.body.leftMap(new RuntimeException(_)))
        .subflatMap(identity)
        .subflatMap(decode[T](_))
      // TODO render error properly
    } yield node).value.toIO.flatMap(IO.fromEither)

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping(): IO[Node[C]] =
    call[Node[C]](_.post, uri"http://$hostname:$port/kad/ping")

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, neighbors: StatusCode): IO[Seq[Node[C]]] =
    call[Seq[Node[C]]](_.post, uri"http://$hostname:$port/kad/lookup?key=${key.asBase58}&n=$neighbors")

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, neighbors: StatusCode): IO[Seq[Node[C]]] =
    call[Seq[Node[C]]](
      _.get,
      uri"http://$hostname:$port/kad/lookup?key=${key.asBase58}&n=$neighbors&awayFrom=${moveAwayFrom.asBase58}"
    )
}
