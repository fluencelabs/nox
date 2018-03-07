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

import java.nio.ByteBuffer

import cats.Applicative
import cats.kernel.Eq
import fluence.codec.Codec
import scodec.bits.ByteVector

import scala.language.higherKinds

/** Any data hash */
case class Hash(bytes: Array[Byte]) extends AnyVal {

  def copy: Hash = Hash(BytesOps.copyOf(bytes))

  override def toString: String = ByteVector.view(bytes).toString

}

object Hash {

  def empty: Hash = Hash(Array.emptyByteArray)

  implicit def hashCodec[F[_] : Applicative]: Codec[F, Hash, Array[Byte]] = Codec.pure(_.bytes, b ⇒ Hash(b))

  implicit val hashEq: Eq[Hash] = {
    (k1, k2) ⇒ ByteBuffer.wrap(k1.bytes).equals(ByteBuffer.wrap(k2.bytes))
  }

  implicit class HashOps(originHash: Hash) {

    def isEmpty: Boolean = originHash.bytes.isEmpty

    def concat(hash: Hash): Hash = Hash(Array.concat(originHash.bytes, hash.bytes))

    def concat(hash: Array[Hash]): Hash = Hash(Array.concat(originHash.bytes, hash.flatMap(_.bytes)))

  }

  implicit class ArrayHashOps(hashes: Array[Hash]) {

    /**
     * Returns updated copy of hash array with the updated element for ''insIdx'' index.
     * We choose variant with array copying for prevent changing input parameters.
     * Work with mutable structures is more error-prone. It may be changed in the future by performance reason.
     */
    def rewriteValue(newElement: Hash, idx: Int): Array[Hash] = {
      val newArray = hashes.clone()
      newArray(idx) = newElement
      newArray
    }

    /**
     * Returns updated copy of hash array with the inserted element at the specified position(''insIdx'').
     * Current array will grow up by one.
     */
    def insertValue(newElement: Hash, idx: Int): Array[Hash] = {
      val newArray = new Array[Hash](hashes.length + 1)
      // copying init of array
      Array.copy(hashes, 0, newArray, 0, idx)
      // insert element to the empty slot
      newArray(idx) = newElement
      // copying tail of array
      Array.copy(hashes, idx, newArray, idx + 1, hashes.length - idx)

      newArray
    }

  }

}

