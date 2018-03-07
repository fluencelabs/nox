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

import cats.Show

/**
 * ''Show'' type class implementations for common btree entities.
 */
object BTreeCommonShow {

  implicit val showBytes: Show[Array[Byte]] = (b: Array[Byte]) ⇒ new String(b)
  implicit val showLong: Show[Long] = Show.fromToString[Long]

  implicit def showArray[T](implicit sb: Show[T]): Show[Array[T]] =
    Show.show((array: Array[T]) ⇒ array.map(sb.show).mkString("[", ",", "]"))

  implicit def showArrayOfBytes(implicit sk: Show[Array[Byte]]): Show[Array[Array[Byte]]] = showArray(showBytes)
  implicit def showArrayOfLong(implicit svr: Show[Array[Long]]): Show[Array[Long]] = showArray(showLong)

  implicit def showPutDetails(implicit svr: Show[ValueRef], sh: Show[Hash]): Show[ClientPutDetails] = {
    Show.show((pd: ClientPutDetails) ⇒
      s"ClientPutDetails(${pd.key}, ${sh.show(pd.valChecksum)}, ${pd.searchResult})"
    )
  }

}
