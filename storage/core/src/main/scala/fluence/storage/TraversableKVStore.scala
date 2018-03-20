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

import scala.language.higherKinds

/**
  * Traversable Key-value storage api interface.
  *
  * @tparam FS A box for returning stream(cursor) of value
  * @tparam K The type of keys
  * @tparam V The type of stored values
  */
trait TraversableKVStore[FS[_], K, V] {

  /**
    * Return all pairs (K, V) for specified dataSet.
    *
    * @return Cursor for found pairs (K,V)
    */
  def traverse(): FS[(K, V)]
}
