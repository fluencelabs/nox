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

package fluence.kvstore

import cats.Monad
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import fluence.kvstore.KVStore.TraverseOp
import fluence.kvstore.ops._

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds

/**
 * Top type for in memory kvStore implementation,
 * just holds kvStore state.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 */
private sealed trait InMemoryKVStore[K, V] extends KVStore {

  protected def data: TrieMap[K, V]

}

object InMemoryKVStore {

  /**
   * Allows reading keys and values from KVStore.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  private trait InMemoryKVStoreRead[K, V] extends InMemoryKVStore[K, V] with KVStoreRead[K, V] { self ⇒

    /**
     * Returns lazy ''get'' representation (see [[Operation]])
     *
     * @param key Search key
     */
    override def get(key: K): Operation[Option[V]] = new Operation[Option[V]] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Option[V]] =
        EitherT(IO(data.get(key)).attempt.to[F])
          .leftMap(err ⇒ StoreError.forGet(key, Some(err)))

    }

    /**
     * Returns lazy ''traverse'' representation (see [[TraverseOperation]])
     */
    override def traverse: TraverseOp[K, V] = new TraverseOp[K, V] {

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
  private trait InMemoryKVStoreWrite[K, V] extends InMemoryKVStore[K, V] with KVStoreWrite[K, V] { self ⇒

    /**
     * Returns lazy ''put'' representation (see [[Operation]])
     *
     * @param key The specified key to be inserted
     * @param value The value associated with the specified key
     */
    override def put(key: K, value: V): Operation[Unit] = new Operation[Unit] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        EitherT(IO(data.put(key, value)).attempt.to[F])
          .leftMap(err ⇒ StoreError.forPut(key, value, Some(err)))
          .map(_ ⇒ ())

    }

    /**
     * Returns lazy ''remove'' representation (see [[Operation]])
     *
     * @param key The specified key to be inserted
     */
    override def remove(key: K): Operation[Unit] = new Operation[Unit] {

      override def run[F[_]: Monad: LiftIO]: EitherT[F, StoreError, Unit] =
        EitherT(IO(data.remove(key)).attempt.to[F])
          .leftMap(err ⇒ StoreError.forRemove(key, Some(err)))
          .map(_ ⇒ ())

    }

  }

  /**
   * Create in memory [[ReadWriteKVStore]].
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  def apply[K, V]: ReadWriteKVStore[K, V] =
    new TrieMapKVStore[K, V] with InMemoryKVStoreRead[K, V] with InMemoryKVStoreWrite[K, V] with ReadWriteKVStore[K, V]

  /**
   * Create in memory [[ReadWriteKVStore]] with snapshot functionality.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   */
  def withSnapshots[K, V]: ReadWriteKVStore[K, V] with Snapshot[KVStoreRead[K, V]] = {
    new TrieMapKVStore[K, V] with InMemoryKVStoreRead[K, V] with InMemoryKVStoreWrite[K, V] with ReadWriteKVStore[K, V]
    with Snapshot[KVStoreRead[K, V]] {
      override def createSnapshot[F[_]: LiftIO](): F[KVStoreRead[K, V]] =
        IO[KVStoreRead[K, V]](new TrieMapKVStore(data.snapshot()) with InMemoryKVStoreRead[K, V]).to[F]
    }
  }

  private abstract class TrieMapKVStore[K, V](map: TrieMap[K, V] = TrieMap.empty[K, V]) extends InMemoryKVStore[K, V] {
    protected val data: TrieMap[K, V] = map
  }

}
