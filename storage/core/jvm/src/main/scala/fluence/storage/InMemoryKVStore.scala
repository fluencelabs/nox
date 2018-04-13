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
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.{~>, Applicative, Monad, MonadError}
import fluence.store._

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds
import scala.util.Try

/**
 * Top type for in memory kvStore implementation,
 * just holds kvStore state.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 */
private sealed trait InMemoryKVStore[K, V] extends KVStorage {

  protected def data: TrieMap[K, V]

}

/**
 * Lazy representation for getting value by specified key.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 */
private trait InMemoryKVStoreRead[K, V] extends InMemoryKVStore[K, V] with KVStoreRead[K, V, StoreError] { self ⇒

  /**
   * Returns lazy ''get'' representation (see [[Get]])
   */
  override def get: Get[K, V, StoreError] = new Get[K, V, StoreError] {

    override def run[F[_]: Monad](key: K): EitherT[F, StoreError, Option[V]] =
      EitherT.fromEither(
        Try(data.get(key)).toEither.left.map(err ⇒ StoreError.getError(key, Some(err)))
      )

    override def runUnsafe(key: K): Option[V] =
      data.get(key)

  }

  /**
   * Returns lazy ''traverse'' representation (see [[Traverse]])
   */
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

}

private trait InMemoryKVStoreWrite[K, V] extends InMemoryKVStore[K, V] with KVStoreWrite[K, V, StoreError] { self ⇒

  override def put[F[_]: Monad](key: K, value: V): F[Either[StoreError, Unit]] =
    Try(data.put(key, value)).toEither.left
      .map(err ⇒ StoreError.putError(key, value, Some(err)))
      .right
      .map(_ ⇒ ())
      .pure[F]

  override def remove[F[_]: Monad](key: K): F[Either[StoreError, Unit]] =
    Try(data.remove(key)).toEither.left
      .map(err ⇒ StoreError.removeError(key, Some(err)))
      .right
      .map(_ ⇒ ())
      .pure[F]

}

object InMemoryKVStore {

  private abstract class TrieMapKVStore[K, V](map: TrieMap[K, V] = TrieMap.empty[K, V]) extends InMemoryKVStore[K, V] {
    protected val data: TrieMap[K, V] = map
  }

  def apply[K, V]: ReadWriteKVStore[K, V, StoreError] =
    new TrieMapKVStore[K, V] with InMemoryKVStoreRead[K, V] with InMemoryKVStoreWrite[K, V]
    with ReadWriteKVStore[K, V, StoreError]

  def withSnapshots[K, V]: ReadWriteKVStore[K, V, StoreError] with Snapshot[KVStoreRead[K, V, StoreError]] = {
    new TrieMapKVStore[K, V] with InMemoryKVStoreRead[K, V] with InMemoryKVStoreWrite[K, V]
    with ReadWriteKVStore[K, V, StoreError] with Snapshot[KVStoreRead[K, V, StoreError]] {
      override def createSnapshot[F[_]: Applicative](): F[KVStoreRead[K, V, StoreError]] = {
        Applicative[F].pure(
          new TrieMapKVStore(data.snapshot()) with InMemoryKVStoreRead[K, V]
        )
      }
    }
  }

  // this MonadError is needed for travers
  implicit def storeMonadError[F[_], E <: StoreError](implicit ME: MonadError[F, Throwable]): MonadError[F, E] =
    new MonadError[F, E] {
      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)
      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)
      override def raiseError[A](e: E): F[A] = ME.raiseError(e)
      override def handleErrorWith[A](fa: F[A])(f: E ⇒ F[A]): F[A] = ME.handleErrorWith(fa) {
        case cf: E ⇒ f(cf)
        case t ⇒ ME.raiseError(t)
      }
      override def pure[A](x: A): F[A] =
        ME.pure(x)
    }

}
