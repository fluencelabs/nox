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
import fluence.crypto.Crypto
import fluence.effects.kvstore.{KVStore, ValueCodecError}
import fluence.kad.protocol.Key
import fluence.log.Log
import scodec.bits.ByteVector

import scala.language.{higherKinds, postfixOps}

/**
 * Local implementation for DhtRpc: store values in the local store, schedule refreshing.
 *
 * @param store Data storage
 * @param metadata Metadata storage
 * @param hasher Values hasher
 * @param scheduleRefresh Callback to schedule refresh of an updated (touched) value
 * @tparam F Effect
 * @tparam V Value; on update, new value is combined with the old one with [[Semigroup.combine]]
 */
class DhtLocalStore[F[_]: Monad: Clock, V: Semigroup](
  store: KVStore[F, Key, Array[Byte]],
  metadata: KVStore[F, Key, DhtValueMetadata],
  hasher: Crypto.Hasher[Array[Byte], ByteVector],
  scheduleRefresh: (Key, DhtValueMetadata) ⇒ F[Unit]
)(implicit codec: PureCodec[V, Array[Byte]])
    extends DhtRpc[F, V] {

  private val timestampT =
    EitherT.right[DhtError](Clock[F].realTime(TimeUnit.SECONDS))

  /**
   * Retrieve the value from node's local storage
   *
   */
  override def retrieve(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, V] =
    store
      .transformValues[V]
      .get(key)
      .leftMap(DhtLocalStoreError(_))
      .subflatMap(
        _.fold[Either[DhtError, V]](Left(DhtValueNotFound(key)))(Right(_))
      )

  /**
   * Kindly ask node to store the value in its local store.
   * Note that remote node may consider not to store the value or to modify it (e.g. combine with a semigroup).
   * You may need to check the value's consistency with a consequent [[retrieve]] call.
   *
   */
  override def store(key: Key, value: V)(implicit log: Log[F]): EitherT[F, DhtError, Unit] =
    for {
      timestamp ← timestampT

      // Ignore all errors while getting old value
      oldValueOpt ← EitherT.right(
        store
          .transformValues[V]
          .get(key)
          .value
          .map(_.toOption.flatten)
      )

      // If the value for the key was previously stored, combine it with the new one
      newValue = oldValueOpt.fold(value)(_ |+| value)

      newBytes ← codec.direct(newValue).leftMap(e ⇒ DhtLocalStoreError(ValueCodecError(e)))

      newHash ← hasher(newBytes).leftMap(DhtCryptError)

      dhtMetadata = DhtValueMetadata(timestamp, newHash)

      _ ← metadata.put(key, dhtMetadata).leftMap(DhtLocalStoreError(_))

      _ ← store.put(key, newBytes).leftMap(DhtLocalStoreError(_))

      _ ← EitherT.right(scheduleRefresh(key, dhtMetadata))
    } yield ()

  /**
   * Retrieve hash of the value, if it is stored
   *
   */
  override def retrieveHash(key: Key)(implicit log: Log[F]): EitherT[F, DhtError, ByteVector] =
    metadata
      .get(key)
      .leftMap(DhtLocalStoreError)
      .subflatMap(
        _.fold[Either[DhtError, ByteVector]](Left(DhtValueNotFound(key)))(m ⇒ Right(m.hash))
      )
}

object DhtLocalStore {

  /**
   * Makes a new DhtLocalStore instance, running all the expected background jobs (namely, refreshing).
   *
   * @param store Local values storage
   * @param metadata Local metadata storage
   * @param hasher Values hasher
   * @tparam F Effect
   * @tparam V Value
   */
  def make[F[_]: Monad: Timer, V: Semigroup](
    store: KVStore[F, Key, Array[Byte]],
    metadata: KVStore[F, Key, DhtValueMetadata],
    hasher: Crypto.Hasher[Array[Byte], ByteVector]
  )(implicit codec: PureCodec[V, Array[Byte]]): Resource[F, DhtLocalStore[F, V]] = {

    // TODO implement refresh scheduling callback
    def scheduleRefresh(key: Key, meta: DhtValueMetadata): F[Unit] =
      Applicative[F].unit

    // TODO schedule refresh for old records by streaming the metadata

    Resource.pure(new DhtLocalStore[F, V](store, metadata, hasher, scheduleRefresh))

  }
}
