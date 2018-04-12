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

import cats.syntax.applicative._
import cats.{~>, Applicative, Monad, MonadError}
import fluence.store._

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds
import scala.util.Try

// todo docs
private sealed trait InMemoryKVStore[K, V] extends KVStorage {

  protected def data: TrieMap[K, V]

}

private trait InMemoryKVStoreRead[K, V] extends InMemoryKVStore[K, V] with KVStoreRead[K, V, StoreError] { self ⇒

  override def get[F[_]: Monad](key: K): F[Either[StoreError, Option[V]]] =
    Try(data.get(key)).toEither.left
      .map(err ⇒ StoreError.getError(key, Some(err)))
      .pure[F]

  override def traverse[FS[_]]()(implicit ME: MonadError[FS, StoreError], liftToFS: Iterator ~> FS): FS[(K, V)] =
    liftToFS(data.iterator)

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
