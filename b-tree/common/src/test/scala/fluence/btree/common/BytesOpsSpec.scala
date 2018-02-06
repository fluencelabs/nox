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

import org.scalatest.{ Matchers, WordSpec }

class BytesOpsSpec extends WordSpec with Matchers {

  "copyOf" should {
    "correct copy byte array" when {
      "array is empty" in {
        BytesOps.copyOf(Array.emptyByteArray) shouldBe Array.emptyByteArray
      }
      "array if filled" in {
        val source = "source".getBytes
        val copy = BytesOps.copyOf(source)
        copy eq source shouldBe false
        copy shouldBe source
      }
    }
    "correct copy array of byte array" when {
      "array is empty" in {
        BytesOps.copyOf(Array.empty[Array[Byte]]) shouldBe Array.empty[Array[Byte]]
      }
      "array if filled" in {
        val source = Array("A".getBytes, "B".getBytes, "C".getBytes)
        val copy = BytesOps.copyOf(source)
        copy eq source shouldBe false
        copy shouldBe source
      }
    }
  }

  "rewriteValue" should {
    "throw exception" when {
      "source array is empty" in {
        intercept[ArrayIndexOutOfBoundsException] {
          BytesOps.rewriteValue(Array.empty[Array[Byte]], "A".getBytes, 0) shouldBe Array.empty[Array[Byte]]
        }
      }
      "idx out of bound" in {
        intercept[ArrayIndexOutOfBoundsException] {
          BytesOps.rewriteValue(Array("A".getBytes), "A".getBytes, 10) shouldBe Array.empty[Array[Byte]]
        }
      }
    }

    "copy source array and update value" when {
      "array if filled" in {
        val source = Array("A".getBytes, "B".getBytes, "C".getBytes)
        val updatedSource = BytesOps.rewriteValue(source, "X".getBytes, 1)
        updatedSource eq source shouldBe false
        updatedSource should contain theSameElementsInOrderAs Array("A".getBytes, "X".getBytes, "C".getBytes)
      }
    }
  }

  "insertValue" should {
    "throw exception" when {
      "idx out of bound" in {
        intercept[ArrayIndexOutOfBoundsException] {
          BytesOps.insertValue(Array("A".getBytes), "A".getBytes, 10) shouldBe Array.empty[Array[Byte]]
        }
      }
    }

    "copy source array and insert value" when {
      "source array is empty" in {
        BytesOps.insertValue(Array.empty[Array[Byte]], "A".getBytes, 0) shouldBe Array("A".getBytes)
      }

      "array if filled" in {
        val source = Array("A".getBytes, "B".getBytes, "C".getBytes)
        val updatedSource = BytesOps.insertValue(source, "X".getBytes, 1)
        updatedSource eq source shouldBe false
        updatedSource should contain theSameElementsInOrderAs Array("A".getBytes, "X".getBytes, "B".getBytes, "C".getBytes)
      }
    }
  }

}
