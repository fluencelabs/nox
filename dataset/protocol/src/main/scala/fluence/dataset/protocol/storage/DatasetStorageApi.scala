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
package fluence.dataset.protocol.storage

import scala.language.higherKinds

/**
 * Dataset storage api interface. Main client interface for data manipulation.
 *
 * @tparam F A box for returning value
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait DatasetStorageApi[F[_], K, V] {

  /**
   * Gets stored value for specified key.
   *
   * @param key The key retrieve the value.
   * @return returns found value, None if nothing was found.
   */
  def get(key: K): F[Option[V]]

  /**
   * Puts key value pair (K, V). Update existing value if it's present.
   *
   * @param key The specified key to be inserted
   * @param value The value associated with the specified key
   * @return if old value was overridden returns old value, None otherwise.
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
