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
 * Remotely-accessible interface to value storage.
 *
 * @tparam F A box for returning value
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait ValueStorageRpc[F[_], K, V] {

  /**
   * Gets stored value for specified key.
   *
   * @param key The key retrieve the value.
   */
  def get(key: K): F[V]

  /**
   * Puts key value pair (K, V). Update existing value if it's present.
   *
   * @param value The value associated with the specified key
   * @return generated key corresponding to saved the value
   */
  def put(value: V): F[K]

  /**
   * Removes pair (K, V) for specified key.
   *
   * @param key The key to delete within database
   */
  def remove(key: K): F[Unit]

}
