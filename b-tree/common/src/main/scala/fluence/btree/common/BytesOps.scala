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

package fluence.btree.common

import java.util

/**
 * Frequently used operations with bytes.
 */
object BytesOps {

  /** Returns copy of bytes. */
  def copyOf(bytes: Array[Byte]): Array[Byte] =
    util.Arrays.copyOf(bytes, bytes.length)

  /** Returns shallow copy of specified array. Note that, this method doesn't copy array elements! */
  def copyOf(array: Array[Array[Byte]]): Array[Array[Byte]] =
    util.Arrays.copyOf(array, array.length)

  /**
   * Returns updated shallow copy of array with the updated element for ''insIdx'' index.
   * We choose variant with array copying for prevent changing input parameters.
   * Work with mutable structures is more error-prone. It may be changed in the future by performance reason.
   */
  def rewriteValue(array: Array[Array[Byte]], newElement: Array[Byte], idx: Int): Array[Array[Byte]] = {
    val newArray = copyOf(array)
    newArray(idx) = newElement
    newArray
  }

  /**
   * Returns updated shallow copy of array with the inserted element at the specified position(''insIdx'').
   * Current array will grow up by one.
   */
  def insertValue(array: Array[Array[Byte]], newElement: Array[Byte], idx: Int): Array[Array[Byte]] = {
    val newArray = util.Arrays.copyOf(array, array.length + 1)
    Array.copy(array, idx, newArray, idx + 1, array.length - idx)
    newArray(idx) = newElement
    newArray
  }

}
