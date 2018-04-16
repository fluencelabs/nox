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

import fluence.kvstore.{KVStore, StoreError}

import scala.language.higherKinds

trait Get[V, E <: StoreError] extends Operation[Option[V], E]

object Get {

  /**
   * Contract for obtaining values by key. In other words ''mixin'' with ''get'' functionality.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   * @tparam E A type for any storage errors
   */
  trait KVStoreGet[K, V, E <: StoreError] extends KVStore {

    /**
     * Returns lazy ''get'' representation (see [[Get]])
     *
     * @param key Search key
     */
    def get(key: K): Get[V, StoreError]

  }

}
