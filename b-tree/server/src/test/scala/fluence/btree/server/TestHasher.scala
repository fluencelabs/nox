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

package fluence.btree.server

import fluence.btree.core.Hash
import fluence.crypto.hash.{CryptoHasher, TestCryptoHasher}

class TestHasher(hasher: CryptoHasher[Array[Byte], Array[Byte]]) extends CryptoHasher[Array[Byte], Hash] {

  override def hash(msg: Array[Byte]): Hash = Hash(hasher.hash(msg))

  override def hash(msg1: Array[Byte], msgN: Array[Byte]*): Hash = Hash(hasher.hash(msg1, msgN: _*))

}

object TestHasher {
  def apply(hasher: CryptoHasher[Array[Byte], Array[Byte]] = TestCryptoHasher): TestHasher = new TestHasher(hasher)
}
