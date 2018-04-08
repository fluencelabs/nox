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

import java.nio.ByteBuffer

import cats.~>
import monix.eval.Task
import monix.execution.ExecutionModel
import monix.execution.schedulers.TestScheduler
import monix.reactive.Observable
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._

class TrieMapKVStoreSpec extends WordSpec with Matchers with ScalaFutures {

  type Key = Array[Byte]
  type Value = Array[Byte]

  implicit def wrapBytes(bytes: Array[Byte]): ByteBuffer = ByteBuffer.wrap(bytes)

  "TrieMapKVStore" should {
    "performs all operations correctly" in {

      implicit val testScheduler: TestScheduler = TestScheduler(ExecutionModel.AlwaysAsyncExecution)

      val store = TrieMapKVStore.withTraverse[Task, Observable, ByteBuffer, Array[Byte]](new (Iterator ~> Observable) {
        override def apply[A](fa: Iterator[A]): Observable[A] = Observable.fromIterator(fa)
      })

      val key1 = "key1".getBytes()
      val val1 = "val1".getBytes()
      val key2 = "key2".getBytes()
      val val2 = "val2".getBytes()
      val newVal2 = "new val2".getBytes()

      // check write and read

      val case1 = Task
        .sequence(
          Seq(
            store.get(key1).attempt.map(_.toOption),
            store.put(key1, val1),
            store.get(key1)
          )
        )
        .runAsync

      testScheduler.tick(5.seconds)

      val case1Result = case1.futureValue
      check(case1Result, Seq(None, (), val1))

      // check update

      val case2 = Task
        .sequence(
          Seq(
            store.put(key2, val2),
            store.get(key2),
            store.put(key2, newVal2),
            store.get(key2)
          )
        )
        .runAsync

      testScheduler.tick(5.seconds)

      val case2Result = case2.futureValue
      check(case2Result, Seq((), val2, (), newVal2))

      // check delete

      val case3 = Task
        .sequence(
          Seq(
            store.get(key1),
            store.remove(key1),
            store.get(key1).attempt.map(_.toOption)
          )
        )
        .runAsync

      testScheduler.tick(5.seconds)

      val case3Result = case3.futureValue
      check(case3Result, Seq(val1, (), None))

      // check traverse

      val manyPairs: Seq[(Key, Value)] = 1 to 100 map { n ⇒
        s"key$n".getBytes() → s"val$n".getBytes()
      }
      val inserts = manyPairs.map { case (k, v) ⇒ store.put(k, v) }

      val case4 = Task.sequence(inserts).flatMap(_ ⇒ store.traverse().toListL).runAsync

      testScheduler.tick(5.seconds)

      val traverseResult = case4.futureValue
      bytesToStr(traverseResult.map {
        case (bb, v) ⇒ bb.array() -> v
      }) should contain theSameElementsAs bytesToStr(manyPairs)

    }
  }

  private def bytesToStr(bytes: Seq[(Array[Byte], Array[Byte])]): Seq[(String, String)] = {
    bytes.map { case (k, v) ⇒ new String(k) → new String(v) }
  }

  private def check(result: Seq[Any], expected: Seq[Any]): Unit =
    result.map {
      case Some(v) ⇒ v
      case x ⇒ x
    } should contain theSameElementsInOrderAs expected

}
