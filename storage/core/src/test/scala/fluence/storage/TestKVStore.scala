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

import cats.data.EitherT
import cats.syntax.flatMap._
import cats.{~>, Monad, MonadError}
import fluence.kvstore.ops.{Get, Put, Remove, Traverse}
import fluence.kvstore.{KVStorage, ReadWriteKVStore, StoreError}

import scala.collection.mutable
import scala.language.higherKinds
import scala.util.Try

class TestKVStore[K, V] extends KVStorage with ReadWriteKVStore[K, V, StoreError] {

  private val data = mutable.Map.empty[K, V]

  override def get(key: K): Get[K, V, StoreError] =
    new Get[K, V, StoreError] {

      override def run[F[_]: Monad]: EitherT[F, StoreError, Option[V]] =
        EitherT.fromEither(
          Try(data.get(key)).toEither.left.map(err ⇒ StoreError.getError(key, Some(err)))
        )
      override def runUnsafe(): Option[V] =
        data.get(key)

    }

  override def traverse: Traverse[K, V, StoreError] = new Traverse[K, V, StoreError] {

    override def run[FS[_]: Monad](
      implicit FS: MonadError[FS, StoreError],
      liftIterator: ~>[Iterator, FS]
    ): FS[(K, V)] =
      FS.fromEither {
        Try(liftIterator(data.iterator)).toEither.left.map(err ⇒ StoreError.traverseError(Some(err)))
      }.flatten

    override def runUnsafe: Iterator[(K, V)] =
      data.iterator

  }

  override def put(key: K, value: V): Put[K, V, StoreError] = new Put[K, V, StoreError] {

    override def run[F[_]: Monad]: EitherT[F, StoreError, Unit] =
      // format: off
      EitherT.fromEither {
        Try(data.put(key, value))
          .toEither
          .left.map(err ⇒ StoreError.putError(key, value, Some(err)))
          .right.map(_ ⇒ ())
      }
    // format: on

    override def runUnsafe(): Unit =
      data.put(key, value)

  }

  override def remove(key: K): Remove[K, StoreError] = new Remove[K, StoreError] {

    override def run[F[_]: Monad]: EitherT[F, StoreError, Unit] =
      // format: off
      EitherT.fromEither {
        Try(data.remove(key))
          .toEither
          .left.map(err ⇒ StoreError.removeError(key, Some(err)))
          .right.map(_ ⇒ ())
      }
    // format: on

    override def runUnsafe(): Unit =
      data.remove(key)

  }

}
