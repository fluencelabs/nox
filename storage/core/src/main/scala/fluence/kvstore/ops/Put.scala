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

import fluence.kvstore.{KVStorage, StoreError}

import scala.language.higherKinds

/**
 * Lazy representation for putting key and value.
 *
 * @tparam E A type for any storage errors
 */
trait Put[E <: StoreError] extends Operation[Unit, E]

object Put {

  /**
   * Contract for putting key and value into KVStore.
   * In other words ''mixin'' with ''put'' functionality.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   * @tparam E A type for any storage errors
   */
  trait KVStorePut[K, V, E <: StoreError] extends KVStorage {

    /**
     * Returns lazy ''put'' representation (see [[Put]])
     *
     * @param key The specified key to be inserted
     * @param value The value associated with the specified key
     */
    def put(key: K, value: V): Put[E]

  }

}
