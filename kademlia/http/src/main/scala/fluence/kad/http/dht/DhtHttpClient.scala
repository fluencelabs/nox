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
import fluence.codec.PureCodec
import fluence.kad.dht.{Dht, DhtError, DhtRemoteError}
import fluence.kad.protocol.Key
import fluence.log.Log

import scala.language.higherKinds

class DhtHttpClient[F[_]: Effect, V](
  hostname: String,
  port: Short,
  prefix: String
)(implicit
  sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing],
  valueCodec: PureCodec[V, Array[Byte]])
    extends Dht[F, V] {

  private def uri(key: Key) = uri"http://$hostname:$port/$prefix/${key.asBase58}"

  override def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V] =
    sttp
      .response(asByteArray)
      .get(uri(key))
      .send()
      .map(_.body.leftMap[Throwable](new RuntimeException(_)))
      .subflatMap[Throwable, Array[Byte]](identity)
      .leftMap(e ⇒ DhtRemoteError("Retrieve request errored", Some(e)))
      .flatMap(body ⇒ valueCodec.inverse(body).leftMap(ce ⇒ DhtRemoteError("Invalid response", Some(ce))))

  override def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    valueCodec
      .direct(value)
      .leftMap(ce ⇒ DhtRemoteError("Cannot encode value to store it", Some(ce)))
      .flatMap(
        data ⇒
          sttp
            .body(data)
            .put(uri(key))
            .send()
            .map(_.body.leftMap[Throwable](new RuntimeException(_)))
            .subflatMap[Throwable, String](identity)
            .leftMap(e ⇒ DhtRemoteError("Store request errored", Some(e)): DhtError)
      )
      .void
}
