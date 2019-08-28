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
import cats.effect.Effect
import cats.syntax.either._
import cats.syntax.functor._
import com.softwaremill.sttp._
import fluence.crypto.Crypto
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import fluence.kad.contact.UriContact
import fluence.kad.{KadRemoteError, KadRpcError}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import fluence.log.Log
import io.circe.{Decoder, DecodingFailure}
import io.circe.parser._
import scodec.bits.ByteVector

import scala.language.higherKinds

class KademliaHttpClient[F[_]: Effect: SttpEffect, C](hostname: String, port: Short, auth: String)(
  implicit readNode: Crypto.Func[String, Node[C]]
) extends KademliaRpc[F, C] {

  private val authB64 = UriContact.Schema + " " + ByteVector(auth.getBytes).toBase64

  // TODO: do not drop cause
  private implicit val decodeNode: Decoder[Node[C]] =
    _.as[String].flatMap(readNode.runEither[Id](_).leftMap(ce ⇒ DecodingFailure(ce.message, Nil)))

  private def call[T: Decoder](call: sttp.type ⇒ Uri ⇒ Request[String, Nothing], uri: Uri)(
    implicit log: Log[F]
  ): EitherT[F, KadRpcError, T] =
    for {
      _ ← Log.eitherT[F, KadRpcError].trace(s"Calling Remote: $uri")
      value <- call(sttp)(uri)
        .header(HeaderNames.Authorization, authB64)
        .send()
          .decodeBody(decode[T](_))
        .leftSemiflatMap[KadRpcError](
          t ⇒ log.warn("Errored when calling remote", t) as KadRemoteError("Errored when calling remote", t)
        )
    } yield value

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping()(implicit log: Log[F]): EitherT[F, KadRpcError, Node[C]] =
    call[Node[C]](_.post, uri"http://$hostname:$port/kad/ping")

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, neighbors: StatusCode)(implicit log: Log[F]): EitherT[F, KadRpcError, Seq[Node[C]]] =
    call[Seq[Node[C]]](_.get, uri"http://$hostname:$port/kad/lookup?key=${key.asBase58}&n=$neighbors")

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, neighbors: StatusCode)(
    implicit log: Log[F]
  ): EitherT[F, KadRpcError, Seq[Node[C]]] =
    call[Seq[Node[C]]](
      _.get,
      uri"http://$hostname:$port/kad/lookup?key=${key.asBase58}&n=$neighbors&awayFrom=${moveAwayFrom.asBase58}"
    )
}
