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
import cats.effect.{Effect, Resource, Sync, Timer}
import cats.kernel.Semigroup
import com.softwaremill.sttp.SttpBackend
import fluence.effects.kvstore.KVStore
import fluence.kad.Kademlia
import fluence.kad.contact.UriContact
import fluence.kad.dht.{Dht, DhtLocalStore, DhtRpc, DhtValueMetadata}
import fluence.kad.protocol.Key
import fluence.log.Log
import io.circe.{Decoder, Encoder}

import scala.language.higherKinds

case class DhtHttpNode[F[_], V](
  http: DhtHttp[F],
  dht: KVStore[F, Key, V]
)

object DhtHttpNode {

  /**
   * Make everything required to participate in and use the Kademlia Distributed Hash Table
   *
   * @param prefix Name of this DHT, will be used in URI
   * @param store Store for the values
   * @param metadata Store for the [[DhtValueMetadata]]
   * @param kad Kademlia network
   * @param conf DHT configuration that's going to be used for [[KVStore]] ops; see [[Dht]] for details
   * @tparam F Effect
   * @tparam V Value: Semigroup to merge several values; Encoder/Decoder for HTTP API serialization
   */
  def make[F[_]: Sync: Effect: Timer: Log, V: Semigroup: Encoder: Decoder](
    prefix: String,
    store: Resource[F, KVStore[F, Array[Byte], V]],
    metadata: Resource[F, KVStore[F, Array[Byte], Array[Byte]]],
    kad: Kademlia[F, UriContact],
    conf: Dht.Conf = Dht.Conf()
  )(implicit
    sttpBackend: SttpBackend[EitherT[F, Throwable, ?], Nothing]): Resource[F, DhtHttpNode[F, V]] =
    for {
      s ← store
      m ← metadata
      local ← DhtLocalStore.make(s.transformKeys[Key], m.transform[Key, DhtValueMetadata])
    } yield {
      val http = DhtHttp(prefix, local)

      def rpc(contact: UriContact): DhtRpc[F, V] = new DhtHttpClient[F, V](
        contact.host,
        contact.port,
        prefix
      )

      val dht: KVStore[F, Key, V] = new Dht(kad, rpc, conf)

      DhtHttpNode(http, dht)
    }
}
