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

package fluence.effects.kvstore

import cats.Monad
import cats.data.EitherT
import fluence.codec.PureCodec

import scala.language.higherKinds

abstract class KVStore[F[_]: Monad, K, V] {
  self ⇒

  /**
   * Get the value for a key
   *
   * @param key Key
   * @return None if there's no value for the key
   */
  def get(key: K): EitherT[F, KVReadError, Option[V]]

  /**
   * Put a value for a key
   *
   * @param key Key
   * @param value Value
   */
  def put(key: K, value: V): EitherT[F, KVWriteError, Unit]

  /**
   * Remove a value for a key. Does nothing if the key is not stored
   *
   * @param key Key
   */
  def remove(key: K): EitherT[F, KVWriteError, Unit]

  /**
   * Streams all key-value pairs. Should not block other reads/writes
   *
   */
  def stream: fs2.Stream[F, (K, V)]

  /**
   * Apply a codec to KVStore values, changing the type
   *
   * @param codec Codec
   * @tparam VV New values type
   * @return Updated KVStore
   */
  def transformValues[VV](implicit codec: PureCodec[V, VV]): KVStore[F, K, VV] = new KVStore[F, K, VV] {
    override def get(key: K): EitherT[F, KVReadError, Option[VV]] =
      self.get(key).flatMap {
        case Some(v) ⇒
          codec
            .direct[F](v)
            .bimap(
              ValueCodecError,
              Option(_)
            )
        case None ⇒
          EitherT.pure(None)
      }

    override def put(key: K, value: VV): EitherT[F, KVWriteError, Unit] =
      codec
        .inverse(value)
        .leftMap(ValueCodecError)
        .flatMap(self.put(key, _))

    override def remove(key: K): EitherT[F, KVWriteError, Unit] =
      self.remove(key)

    override def stream: fs2.Stream[F, (K, VV)] =
      self.stream
        .evalMap[F, Either[KVReadError, (K, VV)]] {
          case (key, value) ⇒
            codec
              .direct(value)
              .bimap(
                ValueCodecError(_): KVReadError,
                key -> _
              )
              .value
        }
        .collect {
          case Right(kvv) ⇒ kvv
        }
  }

  /**
   * Apply a codec to KVStore keys, chainging its type
   *
   * @param codec Codec
   * @tparam KK New keys type
   */
  def transformKeys[KK](implicit codec: PureCodec[KK, K]): KVStore[F, KK, V] = new KVStore[F, KK, V] {
    override def get(key: KK): EitherT[F, KVReadError, Option[V]] =
      codec
        .direct(key)
        .leftMap(KeyCodecError)
        .flatMap(self.get)

    override def put(key: KK, value: V): EitherT[F, KVWriteError, Unit] =
      codec
        .direct(key)
        .leftMap(KeyCodecError)
        .flatMap(self.put(_, value))

    override def remove(key: KK): EitherT[F, KVWriteError, Unit] =
      codec
        .direct(key)
        .leftMap(KeyCodecError)
        .flatMap(self.remove)

    override def stream: fs2.Stream[F, (KK, V)] =
      self.stream
        .evalMap[F, Either[KVReadError, (KK, V)]] {
          case (key, value) ⇒
            codec
              .inverse(key)
              .bimap(
                KeyCodecError(_): KVReadError,
                _ -> value
              )
              .value
        }
        .collect {
          case Right(kkv) ⇒ kkv
        }
  }

  /**
   * Apply codecs both to keys and values, changing their types.
   *
   * @param keysCodec Keys codec
   * @param valuesCodec Values codec
   * @tparam KK New keys type
   * @tparam VV New values type
   * @return Transformed KVStore
   */
  def transform[KK, VV](implicit keysCodec: PureCodec[KK, K], valuesCodec: PureCodec[V, VV]): KVStore[F, KK, VV] =
    self.transformKeys[KK].transformValues[VV]
}
