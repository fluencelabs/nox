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

package fluence.kvstore.ops

import cats.effect.LiftIO
import cats.{~>, Monad}

import scala.language.higherKinds

/**
 * Lazy representation for traversing all values.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 */
trait TraverseOperation[K, V] {

  /**
   * Returns FS stream of all pairs in current key-value store.
   *
   * @param liftIterator Creates FS stream from [[Iterator]]
   *
   * @tparam FS User defined type of stream with monadError
   */
  def run[FS[_]: Monad: LiftIO](implicit liftIterator: Iterator ~> FS): FS[(K, V)]

  /**
   * Returns [[Iterator]] with all key-value pairs for current KVStore,
   * '''throw the error if it happens'''. Intended to be used '''only in tests'''.
   */
  // todo find a way to express this method via 'run' and make it final
  def runUnsafe: Iterator[(K, V)]

}
