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

package fluence.dataset.client

import scala.language.higherKinds

/**
 * Dataset storage api interface. Main client interface for data manipulation.
 * TODO: consider removing it, as it seems to be useless
 *
 * @tparam F A box for returning value
 * @tparam FS A type of stream for returning key-value pairs
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait ClientDatasetStorageApi[F[_], FS[_], K, V] {

  /**
   * Gets stored value for specified key.
   *
   * @param key The key retrieve the value.
   * @return returns found value, None if nothing was found.
   */
  def get(key: K): F[Option[V]]

  /**
   * Fetches stored key-value pairs for specified key range as stream.
   *
   * @param from Plain text key, start of range.
   * @param to   Plain text key, end of range.
   * @return returns stream of found key-value pairs.
   */
  def range(from: K, to: K): FS[(K, V)]

  /**
   * Puts key value pair (K, V). Update existing value if it's present.
   *
   * @param key The specified key to be inserted
   * @param value The value associated with the specified key
   * @return returns old value if old value was overridden, None otherwise.
   */
  def put(key: K, value: V): F[Option[V]]

  /**
   * Removes pair (K, V) for specified key.
   *
   * @param key The key to delete within database
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  def remove(key: K): F[Option[V]]

}
