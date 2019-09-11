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

import cats.data.EitherT
import cats.effect.Effect
import cats.syntax.either._
import cats.syntax.functor._
import com.softwaremill.sttp._
import fluence.effects.sttp.SttpEffect
import fluence.effects.sttp.syntax._
import fluence.kad.dht.{DhtError, DhtRemoteError, DhtRpc, DhtValueNotFound}
import fluence.kad.protocol.Key
import fluence.log.Log
import io.circe.{Decoder, Encoder}
import io.circe.parser.decode
import io.circe.syntax._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * [[DhtRpc]] access to a remote server via HTTP transport.
 *
 * @param hostname Remote hostname
 * @param port Remote port
 * @param prefix URL prefix
 * @tparam F Effect
 * @tparam V Value
 */
class DhtHttpClient[F[_]: Effect: SttpEffect, V: Encoder: Decoder](
  hostname: String,
  port: Short,
  prefix: String
) extends DhtRpc[F, V] {

  private def uri(key: Key) =
    uri"http://$hostname:$port/$prefix/${key.asBase58}"

  override def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V] =
    sttp
      .get(uri(key))
      .send()
      .decodeBody(decode[V](_))
      // TODO handle errors properly, return DhtValueNotFound on 404
      .leftMap(e ⇒ DhtRemoteError("Retrieve request errored", Some(e)))

  override def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    sttp
      .body(value.asJson.noSpaces)
      .put(uri(key))
      .send()
      .toBody
      .leftMap(e ⇒ DhtRemoteError("Store request errored", Some(e)): DhtError)
      .void

  override def retrieveHash(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, ByteVector] =
    sttp
      .head(uri(key))
      .send()
      .leftMap(e ⇒ DhtRemoteError("Retrieving hash errored", Some(e)))
      .subflatMap(
        resp ⇒
          if (resp.code == StatusCodes.NotFound) Left(DhtValueNotFound(key))
          else
            resp.body
              .bimap[DhtError, Option[ByteVector]](
                err ⇒ DhtRemoteError(s"Retrieve hash request failed: $err", None),
                _ ⇒ resp.header("etag").flatMap(ByteVector.fromBase64(_))
              )
              .flatMap(
                _.fold[Either[DhtError, ByteVector]](Left(DhtValueNotFound(key)))(Right(_))
              )
      )

}
