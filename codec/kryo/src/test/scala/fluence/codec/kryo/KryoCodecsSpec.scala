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

package fluence.codec.kryo

import cats.instances.try_._
import org.scalatest.{Matchers, WordSpec}

import scala.util.Try

class KryoCodecsSpec extends WordSpec with Matchers {

  val testBlob = Array("3".getBytes(), "4".getBytes(), "5".getBytes())
  val testClass = TestClass("one", 2, testBlob)

  private val testCodecs =
    KryoCodecs()
      .add[Array[Array[Byte]]]
      .addCase(classOf[TestClass])
      .build[Try]()

  "encode and decode" should {
    "be inverse functions" when {
      "object defined" in {

        val codec = testCodecs.codec[TestClass]

        val result = codec.encode(testClass).flatMap(codec.decode).get

        result.str shouldBe "one"
        result.num shouldBe 2
        result.blob should contain theSameElementsAs testBlob
      }

      "object is null" in {
        val codec = testCodecs.codec[TestClass]
        val result = codec.encode(null).flatMap(codec.decode)
        result.isFailure shouldBe true
      }
    }
  }

  "encode" should {
    "not write full class name to binary representation" when {
      "class registered" in {
        //val codec = KryoCodec[TestClass](Seq(classOf[TestClass], classOf[Array[Byte]], classOf[Array[Array[Byte]]]), registerRequired = true)
        val codec = testCodecs.codec[TestClass]
        val encoded = codec.encode(testClass).map(new String(_)).get
        val reasonableMaxSize = 20 // bytes
        encoded should not contain "TestClass"
        encoded.length should be < reasonableMaxSize
      }
    }

  }
}

case class TestClass(str: String, num: Long, blob: Array[Array[Byte]])
