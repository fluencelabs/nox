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

package fluence.node.storage

import cats.{ ApplicativeError, Eval, ~> }

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds

class TrieMapKVStore[F[_], K, V](
    private val data: TrieMap[K, V] = TrieMap.empty[K, V]
)(implicit F: ApplicativeError[F, Throwable]) extends KVStore[F, K, V] {

  protected def iterator: Iterator[(K, V)] = data.iterator

  /**
   * Gets stored value for specified key.
   *
   * @param key The key retrieve the value.
   */
  override def get(key: K): F[V] = F.catchNonFatalEval(Eval.later(data(key)))

  /**
   * Puts key value pair (K, V).
   * Update existing value if it's present.
   *
   * @param key   The specified key to be inserted
   * @param value The value associated with the specified key
   */
  override def put(key: K, value: V): F[Unit] = F.catchNonFatalEval(Eval.later(data(key)= value))

  /**
   * Removes pair (K, V) for specified key.
   *
   * @param key The key to delete within database
   */
  override def remove(key: K): F[Unit] = F.catchNonFatalEval(Eval.later(data.remove(key)))
}

object TrieMapKVStore {

  /**
   * Build an in-memory KVStore
   */
  def apply[F[_], K, V]()(implicit F: ApplicativeError[F, Throwable]): KVStore[F, K, V] =
    new TrieMapKVStore[F, K, V]()

  /**
   * Build an in-memory KVStore with TraversableKVStore support
   */
  def withTraverse[F[_], FS[_], K, V](fromIterator: Iterator ~> FS)(implicit F: ApplicativeError[F, Throwable]): KVStore[F, K, V] with TraversableKVStore[FS, K, V] =
    new TrieMapKVStore[F, K, V]() with TraversableKVStore[FS, K, V] {
      /**
       * Return all pairs (K, V) for specified dataSet.
       *
       * @return cursor to founded pairs (K,V)
       */
      override def traverse(): FS[(K, V)] = fromIterator(iterator)
    }

}
