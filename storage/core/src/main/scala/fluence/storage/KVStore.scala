/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.storage

import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.codec.Codec

import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
  * Key-value storage api interface.
  *
  * @tparam K The type of keys
  * @tparam V The type of stored values
  * @tparam F A box for returning value
  */
trait KVStore[F[_], K, V] {

  /**
    * Gets stored value for specified key.
    * @param key The key retrieve the value.
    */
  def get(key: K): F[V]

  /**
    * Puts key value pair (K, V).
    * Update existing value if it's present.
    * @param key The specified key to be inserted
    * @param value The value associated with the specified key
    */
  def put(key: K, value: V): F[Unit]

  /**
    * Removes pair (K, V) for specified key.
    * @param key The key to delete within database
    */
  def remove(key: K): F[Unit]

}

object KVStore {
  case object KeyNotFound extends NoStackTrace

  // Summoner
  def apply[F[_], K, V](implicit store: KVStore[F, K, V]): KVStore[F, K, V] = store

  implicit def transform[F[_]: Monad, K, K1, V, V1](store: KVStore[F, K, V])(
    implicit
    kCodec: Codec[F, K1, K],
    vCodec: Codec[F, V1, V]): KVStore[F, K1, V1] =
    new KVStore[F, K1, V1] {

      /**
        * Gets stored value for specified key.
        *
        * @param key The key retrieve the value.
        */
      override def get(key: K1): F[V1] =
        for {
          k ← kCodec.encode(key)
          v ← store.get(k)
          v1 ← vCodec.decode(v)
        } yield v1

      /**
        * Puts key value pair (K, V).
        * Update existing value if it's present.
        *
        * @param key   The specified key to be inserted
        * @param value The value associated with the specified key
        */
      override def put(key: K1, value: V1): F[Unit] =
        for {
          k ← kCodec.encode(key)
          v ← vCodec.encode(value)
          _ ← store.put(k, v)
        } yield ()

      /**
        * Removes pair (K, V) for specified key.
        *
        * @param key The key to delete within database
        */
      override def remove(key: K1): F[Unit] =
        for {
          k ← kCodec.encode(key)
          _ ← store.remove(k)
        } yield ()
    }
}
