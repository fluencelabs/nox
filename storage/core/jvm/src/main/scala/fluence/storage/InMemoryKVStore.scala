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
import cats.{~>, Applicative, Monad, MonadError}
import fluence.kvstore._
import fluence.kvstore.ops.{Get, Put, Remove, Traverse}

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds
import scala.util.Try

object InMemoryKVStore {

  /**
   * Create in memory [[ReadWriteKVStore]].
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  def apply[K, V]: ReadWriteKVStore[K, V, StoreError] =
    new TrieMapKVStore[K, V] with InMemoryKVStoreRead[K, V] with InMemoryKVStoreWrite[K, V]
    with ReadWriteKVStore[K, V, StoreError]

  /**
   * Create in memory [[ReadWriteKVStore]] with snapshot functionality.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
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
   * Allows reading keys and values from KVStore.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  private trait InMemoryKVStoreRead[K, V] extends InMemoryKVStore[K, V] with KVStoreRead[K, V, StoreError] { self ⇒

    /**
     * Returns lazy ''get'' representation (see [[fluence.kvstore.ops.Get]])
     *
     * @param key Search key
     */
    override def get(key: K): Get[K, V, StoreError] = new Get[K, V, StoreError] {

      override def run[F[_]: Monad]: EitherT[F, StoreError, Option[V]] =
        EitherT.fromEither(
          Try(data.get(key)).toEither.left.map(err ⇒ StoreError.getError(key, Some(err)))
        )

      override def runUnsafe(): Option[V] =
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

  /**
   * Allows writing and removing keys and values from KVStore.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  private trait InMemoryKVStoreWrite[K, V] extends InMemoryKVStore[K, V] with KVStoreWrite[K, V, StoreError] { self ⇒

    /**
     * Returns lazy ''put'' representation (see [[Put]])
     *
     * @param key The specified key to be inserted
     * @param value The value associated with the specified key
     */
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

    /**
     * Returns lazy ''remove'' representation (see [[Remove]])
     *
     * @param key The specified key to be inserted
     */
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

  private abstract class TrieMapKVStore[K, V](map: TrieMap[K, V] = TrieMap.empty[K, V]) extends InMemoryKVStore[K, V] {
    protected val data: TrieMap[K, V] = map
  }

  // this MonadError is needed for travers
  implicit def storeMonadError[F[_]](implicit ME: MonadError[F, Throwable]): MonadError[F, StoreError] =
    new MonadError[F, StoreError] {
      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)
      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)
      override def raiseError[A](e: StoreError): F[A] = ME.raiseError(e)
      override def handleErrorWith[A](fa: F[A])(f: StoreError ⇒ F[A]): F[A] = ME.handleErrorWith(fa) {
        case cf: StoreError ⇒ f(cf)
        case t ⇒ ME.raiseError(t)
      }
      override def pure[A](x: A): F[A] =
        ME.pure(x)
    }

}
