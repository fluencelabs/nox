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
import org.scalatest.{Matchers, WordSpec}
import cats.implicits._
import fluence.EitherTSttpBackend
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import scodec.bits.ByteVector
import cats.syntax.traverse._
import cats.syntax.list._

import scala.concurrent.ExecutionContext.Implicits.global

class LastCachingQueueSpec extends WordSpec with Matchers {
  implicit private val timer = IO.timer(global)
  implicit private val shift = IO.contextShift(global)

  private def queue = LastCachingQueue[IO, VmHash, Long]
  private def vmHash(height: Long) = VmHash(height, ByteVector.empty)

  "LastCachingQueue" should {
    "cache last element" in {
      val len = 10L
      val height = 1L
      (for {
        q <- queue
        _ <- q.enqueue1(vmHash(height))
        elems <- (1L to len).toList.traverse(_ => q.dequeue(height))
      } yield {
        elems.size shouldBe len
        elems.foreach(_ shouldBe height)
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
