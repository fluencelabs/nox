/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine.control

import cats.Traverse
import cats.effect.IO
import cats.instances.list._
import cats.instances.long._
import cats.syntax.traverse._
import fluence.statemachine.api.StateHash
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class LastCachingQueueSpec extends WordSpec with Matchers {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)

  private def queue = LastCachingQueue[IO, StateHash, Long]
  private def vmHash(height: Long) = StateHash(height, ByteVector.empty)

  "LastCachingQueue" should {
    "cache last element" in {
      val len = 10L
      val height = 1L
      (for {
        q <- queue
        _ <- q.enqueue1(vmHash(height))
        elems <- Traverse[List].traverse((1L to len).toList)(_ => q.dequeue(height))
      } yield {
        elems.size shouldBe len
        elems.foreach(_.height shouldBe height)
      }).unsafeRunSync()
    }

    "get element" in {
      val len = 10
      (for {
        q <- queue
        _ <- (1L to len).toList.traverse(h => q.enqueue1(vmHash(h)))
        e5 <- q.dequeue(5)
      } yield {
        e5.height shouldBe 5
      }).unsafeRunSync()
    }
  }
}
