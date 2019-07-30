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

package fluence.kad.dht

import java.util.concurrent.TimeUnit

import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.effect.{Clock, Resource, Timer}
import cats.kernel.Semigroup
import cats.syntax.semigroup._
import cats.syntax.functor._
import fluence.codec.PureCodec
import fluence.effects.kvstore.{KVReadError, KVStore, KVWriteError}
import fluence.kad.dht
import fluence.kad.protocol.Key

import scala.language.higherKinds

class DhtLocalStore[F[_]: Monad: Clock, V: Semigroup](
  store: KVStore[F, Key, DhtValue[V]],
  scheduleRefresh: (Key, DhtValue[V]) ⇒ F[Unit]
) extends KVStore[F, Key, V] {

  private val timestampT =
    EitherT.right[KVWriteError](Clock[F].realTime(TimeUnit.MILLISECONDS))

  override def get(key: Key): EitherT[F, KVReadError, Option[V]] =
    store.get(key).map(_.map(_.value))

  // Internally, combines an old value with the provided one and connects the timestamp to it prior to storing
  override def put(key: Key, value: V): EitherT[F, KVWriteError, Unit] =
    for {
      timestamp ← timestampT

      oldValueOpt ← EitherT.right(
        store
          .get(key)
          .value
          .map(_.toOption.flatten.map(_.value))
      )

      newValue = oldValueOpt.fold(value)(_ |+| value)
      dhtValue = DhtValue[V](newValue, timestamp)

      _ ← store.put(key, dhtValue)

      _ ← EitherT.right(scheduleRefresh(key, dhtValue))
    } yield ()

  override def remove(key: Key): EitherT[F, KVWriteError, Unit] =
    store.remove(key)

  override def stream: fs2.Stream[F, (Key, V)] =
    store.stream.map {
      case (k, v) ⇒ (k, v.value)
    }
}

object DhtLocalStore {

  def make[F[_]: Monad: Timer, V: Semigroup](store: KVStore[F, Key, Array[Byte]])(
    implicit
    valueCodec: PureCodec[Array[Byte], V]
  ): Resource[F, DhtLocalStore[F, V]] = {

    // TODO prepare refresh scheduling

    val dhtStore =
      new dht.DhtLocalStore[F, V](
        store.transformValues[DhtValue[V]],
        (_, _) ⇒ Applicative[F].unit
      )

    // TODO schedule old records for refreshing

    Resource.pure(dhtStore)

  }
}
