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

/**
 * Lazy representation for removing key and value.
 *
 * @tparam E A type for any storage errors
 */
trait Remove[E <: StoreError] extends Operation[Unit, E]

object Remove {

  /**
   * Contract for removing key and value from KVStore.
   * In other words ''mixin'' with ''remove'' functionality.
   *
   * @tparam K A type of search key
   * @tparam E A type for any storage errors
   */
  trait KVStoreRemove[K, E <: StoreError] extends KVStore {

    /**
     * Returns lazy ''remove'' representation (see [[Remove]])
     *
     * @param key The specified key to be inserted
     */
    def remove(key: K): Remove[E]

  }

}
